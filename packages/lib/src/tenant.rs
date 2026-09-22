use crate::error::AppError;
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, DatabaseTransaction, Statement,
    TransactionTrait,
};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct OrganizationContext {
    pub allowed_organizations: Vec<Uuid>,
    pub organization_roles: HashMap<Uuid, String>,
    pub api_key_organization_id: Option<Uuid>,
}

#[derive(Clone)]
pub struct TenantDatabase {
    tenant_db: DatabaseConnection,
    pub context: OrganizationContext,
}

#[must_use = "call commit() to persist changes; dropping the transaction rolls them back"]
pub struct ScopedTenantTransaction {
    tx: DatabaseTransaction,
}

impl TenantDatabase {
    pub fn new(tenant_db: DatabaseConnection, context: OrganizationContext) -> Self {
        Self { tenant_db, context }
    }

    fn allowed_orgs_literal(&self) -> String {
        if self.context.allowed_organizations.is_empty() {
            return "{}".into();
        }
        let joined = self
            .context
            .allowed_organizations
            .iter()
            .map(Uuid::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!("{{{joined}}}")
    }

    pub async fn begin_scoped_transaction(&self) -> Result<ScopedTenantTransaction, AppError> {
        let tx = self.tenant_db.begin().await?;
        let statement = Statement::from_string(
            DatabaseBackend::Postgres,
            format!(
                "SET LOCAL app.allowed_organizations = '{}';",
                self.allowed_orgs_literal()
            ),
        );
        tx.execute(statement).await?;
        Ok(ScopedTenantTransaction { tx })
    }
}

impl ScopedTenantTransaction {
    pub fn connection(&self) -> &DatabaseTransaction {
        &self.tx
    }

    pub async fn commit(self) -> Result<(), AppError> {
        self.tx.commit().await.map_err(Into::into)
    }
}
