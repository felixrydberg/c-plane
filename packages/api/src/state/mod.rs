use crate::config::{Config, load_config};
use crate::errors::AppError;
use crate::services::s3_providers::S3ProviderClient;
use lib::secrets::Client;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection,
    DatabaseTransaction, Statement, TransactionTrait,
};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::{process, time::Duration};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppDatabase(pub DatabaseConnection);

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

pub struct ScopedTenantTransaction {
    tx: DatabaseTransaction,
}

impl AppDatabase {
    pub fn connection(&self) -> &DatabaseConnection {
        &self.0
    }
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
        format!("{{{}}}", joined)
    }

    pub async fn begin_scoped_transaction(&self) -> Result<ScopedTenantTransaction, AppError> {
        let tx = self
            .tenant_db
            .begin()
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

        let literal = self.allowed_orgs_literal();

        let statement = Statement::from_string(
            DatabaseBackend::Postgres,
            format!("SET LOCAL app.allowed_organizations = '{}';", literal),
        );

        tx.execute(statement)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

        Ok(ScopedTenantTransaction { tx })
    }
}

impl ScopedTenantTransaction {
    pub fn connection(&self) -> &DatabaseTransaction {
        &self.tx
    }

    pub async fn commit(self) -> Result<(), AppError> {
        self.tx
            .commit()
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
    }
}

#[derive(Clone)]
pub struct State {
    pub identity_db: AppDatabase,
    pub tenant_db: DatabaseConnection,
    pub config: Config,
    pub s3_providers: S3ProviderClient,
    pub storage_client: reqwest::Client,
}

static STATE: OnceLock<State> = OnceLock::new();

pub async fn create_app_state() -> Result<State, AppError> {
    let config = load_config()?;
    let identity_db = connect_database(&config.identity_database_url, "app_identity").await?;
    let tenant_db = connect_database(&config.tenant_database_url, "app_tenant").await?;

    let secrets = Client::from_env()?;
    let s3_providers =
        S3ProviderClient::new(tenant_db.clone(), secrets.clone(), config.redis_url.clone());
    let storage_client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .read_timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| AppError::Internal(format!("Failed to create storage client: {error}")))?;
    let state = State {
        identity_db: AppDatabase(identity_db),
        tenant_db,
        config,
        s3_providers,
        storage_client,
    };
    STATE
        .set(state)
        .map_err(|_| AppError::Internal("Couldn't set STATE".into()))?;
    Ok(get_app_state())
}

async fn connect_database(
    database_url: &str,
    role_name: &str,
) -> Result<DatabaseConnection, AppError> {
    let mut options = ConnectOptions::new(database_url);
    options.sqlx_logging(false);

    Database::connect(options)
        .await
        .map_err(|err| AppError::Internal(format!("Failed to connect {role_name} database: {err}")))
}

pub fn get_app_state() -> State {
    match STATE.get() {
        Some(v) => v.clone(),
        None => {
            eprintln!("ERROR: get_app_state() called before initialization");
            let backtrace = std::backtrace::Backtrace::capture();
            eprintln!("{backtrace}");
            process::exit(0)
        }
    }
}
