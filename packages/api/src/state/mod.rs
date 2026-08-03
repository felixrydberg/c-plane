use crate::config::{Config, load_config};
use crate::errors::AppError;
use crate::services::external_registry_tokens::ExternalRegistryTokenClient;
use crate::services::s3_providers::S3ProviderClient;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection,
    DatabaseTransaction, Statement, TransactionTrait,
};
use std::process;
use std::sync::OnceLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppDatabase(pub DatabaseConnection);

#[derive(Clone)]
pub struct OrganizationContext {
    pub allowed_organizations: Vec<Uuid>,
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
    pub s3_providers: Option<S3ProviderClient>,
    pub external_registry_tokens: Option<ExternalRegistryTokenClient>,
}

static STATE: OnceLock<State> = OnceLock::new();

pub async fn create_app_state() -> Result<State, AppError> {
    let config = load_config()?;
    let identity_db = connect_database(&config.identity_database_url, "app_identity").await?;
    let tenant_db = connect_database(&config.tenant_database_url, "app_tenant").await?;

    let control_plane = match (
        config.control_plane_url.clone(),
        config.control_plane_service_token.clone(),
    ) {
        (Some(url), Some(token)) => Some((url, token)),
        _ => None,
    };
    let s3_providers = control_plane
        .clone()
        .map(|(url, token)| S3ProviderClient::new(url, token));
    let external_registry_tokens =
        control_plane.map(|(url, token)| ExternalRegistryTokenClient::new(url, token));
    let state = State {
        identity_db: AppDatabase(identity_db),
        tenant_db,
        config,
        s3_providers,
        external_registry_tokens,
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
