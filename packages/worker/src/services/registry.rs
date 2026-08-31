use crate::{Config, Job, Result, message, statement};
use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction};
use serde::Deserialize;
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Deserialize)]
struct GcReport {
    bytes_before: i64,
    bytes_after: i64,
}

pub(super) fn handles(job: &Job) -> bool {
    job.job_type == "registry_gc"
}

pub(super) async fn run(
    database: &DatabaseConnection,
    config: &Config,
    consumer: &str,
    job: &Job,
) -> Result<()> {
    let organization_id = organization_id(job)?;
    let started_at = Utc::now();
    let run_started = Instant::now();
    info!(
        job_id = %job.id,
        %organization_id,
        "registry garbage collection starting"
    );

    let maintenance_started = Instant::now();
    enter_maintenance(database, job.id, organization_id, consumer).await?;
    info!(
        job_id = %job.id,
        %organization_id,
        elapsed_ms = maintenance_started.elapsed().as_millis() as u64,
        "registry entered maintenance"
    );

    let url = format!(
        "{}/internal/organizations/{organization_id}/garbage-collection",
        config.registry_internal_url
    );
    let request_started = Instant::now();
    info!(
        job_id = %job.id,
        %organization_id,
        endpoint = %config.registry_internal_url,
        "requesting registry garbage collection"
    );
    let response = config
        .registry_http
        .post(url)
        .header("x-cplane-token", &config.service_token)
        .header("x-cplane-job-id", job.id.to_string())
        .send()
        .await;
    let outcome: std::result::Result<GcReport, String> = match response {
        Ok(response) if response.status().is_success() => {
            let status = response.status();
            info!(
                job_id = %job.id,
                %organization_id,
                %status,
                elapsed_ms = request_started.elapsed().as_millis() as u64,
                "registry garbage-collection response received"
            );
            response.json().await.map_err(|error| {
                format!("registry returned an invalid garbage-collection report: {error}")
            })
        }
        Ok(response) => {
            let status = response.status();
            let detail = response.text().await.unwrap_or_default();
            let detail = detail.trim();
            warn!(
                job_id = %job.id,
                %organization_id,
                %status,
                elapsed_ms = request_started.elapsed().as_millis() as u64,
                "registry garbage-collection request was rejected"
            );
            if detail.is_empty() {
                Err(format!("registry garbage collection returned {status}"))
            } else {
                Err(format!(
                    "registry garbage collection returned {status}: {detail}"
                ))
            }
        }
        Err(error) => {
            warn!(
                job_id = %job.id,
                %organization_id,
                %error,
                elapsed_ms = request_started.elapsed().as_millis() as u64,
                "registry garbage-collection request failed"
            );
            Err(format!(
                "registry garbage-collection request failed: {error}"
            ))
        }
    };
    let finished_at = Utc::now();
    let (result, error, bytes) = match &outcome {
        Ok(report) => {
            info!(
                job_id = %job.id,
                %organization_id,
                bytes_before = report.bytes_before,
                bytes_after = report.bytes_after,
                bytes_reclaimed = report.bytes_before.saturating_sub(report.bytes_after),
                elapsed_ms = run_started.elapsed().as_millis() as u64,
                "registry garbage collection completed"
            );
            (
                "succeeded",
                None,
                Some((report.bytes_before, report.bytes_after)),
            )
        }
        Err(error) => {
            warn!(
                job_id = %job.id,
                %organization_id,
                %error,
                elapsed_ms = run_started.elapsed().as_millis() as u64,
                "registry garbage collection failed"
            );
            ("failed", Some(error.as_str()), None)
        }
    };
    database
        .execute(statement(
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
    info!(
        job_id = %job.id,
        %organization_id,
        result,
        elapsed_ms = run_started.elapsed().as_millis() as u64,
        "registry garbage-collection run recorded"
    );
    outcome.map_err(message)?;
    Ok(())
}

pub(super) async fn record_completion(transaction: &DatabaseTransaction, job: &Job) -> Result<()> {
    let organization_id = organization_id(job)?;
    let updated = transaction
        .execute(statement(
            "UPDATE managed_registry SET status='active', gc_active_job_id=NULL, updated_at=NOW() WHERE organization_id=$2::uuid AND gc_active_job_id=$1::uuid",
            vec![job.id.into(), organization_id.into()],
        ))
        .await?;
    if updated.rows_affected() != 1 {
        return Err(message(
            "managed Registry no longer owns the garbage-collection job",
        ));
    }

    Ok(())
}

async fn enter_maintenance(
    database: &DatabaseConnection,
    job_id: Uuid,
    organization_id: Uuid,
    consumer: &str,
) -> Result<()> {
    let result = database
        .execute(statement(
            "UPDATE managed_registry registry SET status='maintenance', updated_at=NOW() WHERE organization_id=$2::uuid AND gc_active_job_id=$1::uuid AND EXISTS (SELECT 1 FROM worker_queue job WHERE job.id=$1::uuid AND job.organization_id=$2::uuid AND job.status='running' AND job.locked_by=$3 AND job.lease_expires_at>NOW())",
            vec![
                job_id.into(),
                organization_id.into(),
                consumer.to_owned().into(),
            ],
        ))
        .await?;
    if result.rows_affected() != 1 {
        return Err(message("registry maintenance lease is no longer owned"));
    }
    Ok(())
}

fn organization_id(job: &Job) -> Result<Uuid> {
    job.organization_id
        .ok_or_else(|| message("registry garbage-collection job requires organization_id"))
}
