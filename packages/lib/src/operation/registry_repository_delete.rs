use sea_orm::{DatabaseTransaction, DbErr};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use super::{Context, Operation, Result};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RegistryRepositoryDelete {
    pub project_id: Uuid,
    pub repository_id: Uuid,
}

impl Operation<RegistryRepositoryDelete> {
    pub const QUEUE: &'static str = "maintenance";
    pub const NAME: &'static str = "registry_repository_delete";
    const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

    pub async fn new(
        transaction: &DatabaseTransaction,
        organization_id: Uuid,
        project_id: Uuid,
        repository_id: Uuid,
    ) -> std::result::Result<Self, DbErr> {
        Self::insert(
            transaction,
            Some(organization_id),
            Self::QUEUE,
            Self::NAME,
            None,
            RegistryRepositoryDelete {
                project_id,
                repository_id,
            },
        )
        .await
    }

    pub async fn run(&self, context: &Context<'_>) -> Result<()> {
        let organization_id = self.metadata.organization_id.ok_or_else(|| {
            Box::new(std::io::Error::other(
                "registry repository deletion requires organization_id",
            )) as super::Error
        })?;
        let response = context
            .registry_http
            .delete(format!(
                "{}/internal/organizations/{organization_id}/projects/{}/repositories/{}",
                context.registry_internal_url, self.input.project_id, self.input.repository_id
            ))
            .header("x-cplane-token", context.service_token)
            .header("x-cplane-job-id", self.metadata.id.to_string())
            .timeout(Self::REQUEST_TIMEOUT)
            .send()
            .await?;
        if response.status().is_success() || response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        let status = response.status();
        let detail = response.text().await.unwrap_or_default();
        Err(Box::new(std::io::Error::other(format!(
            "registry repository deletion returned {status}: {}",
            detail.trim()
        ))))
    }
}
