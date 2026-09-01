use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseTransaction, DbErr, Statement};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Context, Operation, Result};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RegistryGc {
    pub trigger: String,
}

#[derive(Deserialize)]
struct GcReport {
    bytes_before: i64,
    bytes_after: i64,
}

impl Operation<RegistryGc> {
    pub const QUEUE: &'static str = "maintenance";
    pub const NAME: &'static str = "registry_gc";

    pub async fn new(
        transaction: &DatabaseTransaction,
        organization_id: Uuid,
        trigger: impl Into<String>,
    ) -> std::result::Result<Self, DbErr> {
        let input = RegistryGc {
            trigger: trigger.into(),
        };
        let operation = Self::insert(
            transaction,
            Some(organization_id),
            Self::QUEUE,
            Self::NAME,
            Some(format!("registry_gc:{organization_id}")),
            input,
        )
        .await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE managed_registry SET gc_active_job_id=$2, updated_at=NOW() WHERE organization_id=$1",
                vec![organization_id.into(), operation.metadata.id.into()],
            ))
            .await?;
        Ok(operation)
    }

    pub async fn run(&self, context: &Context<'_>) -> Result<()> {
        let organization_id = self.metadata.organization_id.ok_or_else(|| {
            Box::new(std::io::Error::other(
                "registry garbage-collection job requires organization_id",
            )) as super::Error
        })?;
        let started_at = Utc::now();
        enter_maintenance(context, self.metadata.id, organization_id).await?;

        let outcome: std::result::Result<GcReport, String> = async {
            let response = context
                .registry_http
                .post(format!(
                    "{}/internal/organizations/{organization_id}/garbage-collection",
                    context.registry_internal_url
                ))
                .header("x-cplane-token", context.service_token)
                .header("x-cplane-job-id", self.metadata.id.to_string())
                .send()
                .await
                .map_err(|error| format!("registry garbage-collection request failed: {error}"))?;
            let status = response.status();
            if !status.is_success() {
                let detail = response.text().await.unwrap_or_default();
                return Err(if detail.trim().is_empty() {
                    format!("registry garbage collection returned {status}")
                } else {
                    format!(
                        "registry garbage collection returned {status}: {}",
                        detail.trim()
                    )
                });
            }
            response.json().await.map_err(|error| {
                format!("registry returned an invalid garbage-collection report: {error}")
            })
        }
        .await;

        let finished_at = Utc::now();
        let (result, error, bytes) = match &outcome {
            Ok(report) => (
                "succeeded",
                None,
                Some((report.bytes_before, report.bytes_after)),
            ),
            Err(error) => ("failed", Some(error.as_str()), None),
        };
        context
            .database
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "INSERT INTO managed_registry_gc_runs (id, organization_id, started_at, finished_at, bytes_before, bytes_after, result, error) VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8)",
                vec![
                    Uuid::new_v4().into(),
                    organization_id.into(),
                    started_at.into(),
                    finished_at.into(),
                    bytes.map(|(before, _)| before).into(),
                    bytes.map(|(_, after)| after).into(),
                    result.into(),
                    error.into(),
                ],
            ))
            .await?;
        outcome
            .map(|_| ())
            .map_err(|error| Box::new(std::io::Error::other(error)) as super::Error)
    }

    pub(super) async fn complete(
        &self,
        transaction: &DatabaseTransaction,
    ) -> std::result::Result<(), DbErr> {
        let organization_id = self.metadata.organization_id.expect("checked by run");
        let updated = transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE managed_registry SET status='active', gc_active_job_id=NULL, updated_at=NOW() WHERE organization_id=$2::uuid AND gc_active_job_id=$1::uuid",
                vec![self.metadata.id.into(), organization_id.into()],
            ))
            .await?;
        if updated.rows_affected() != 1 {
            return Err(DbErr::Custom(
                "managed Registry no longer owns the garbage-collection job".into(),
            ));
        }
        Ok(())
    }
}

async fn enter_maintenance(
    context: &Context<'_>,
    job_id: Uuid,
    organization_id: Uuid,
) -> Result<()> {
    let result = context
        .database
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE managed_registry registry SET status='maintenance', updated_at=NOW() WHERE organization_id=$2::uuid AND gc_active_job_id=$1::uuid AND EXISTS (SELECT 1 FROM worker_queue job WHERE job.id=$1::uuid AND job.organization_id=$2::uuid AND job.status='running' AND job.locked_by=$3 AND job.lease_expires_at>NOW())",
            vec![
                job_id.into(),
                organization_id.into(),
                context.consumer.to_owned().into(),
            ],
        ))
        .await?;
    if result.rows_affected() != 1 {
        return Err(Box::new(std::io::Error::other(
            "registry maintenance lease is no longer owned",
        )));
    }
    Ok(())
}
