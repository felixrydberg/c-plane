use crate::{Config, Job, Result, message};
use sea_orm::{DatabaseConnection, DatabaseTransaction};

mod foundation;
mod registry;

pub(crate) async fn run(
    database: &DatabaseConnection,
    config: &Config,
    consumer: &str,
    job: &Job,
) -> Result<()> {
    match job.job_type.as_str() {
        "registry_gc" => registry::run(database, config, consumer, job).await,
        "foundation_bucket_delete" => foundation::run(database, config, job).await,
        other => Err(message(format!("unsupported job type: {other}"))),
    }
}

pub(crate) async fn prepare_completion(
    database: &DatabaseConnection,
    config: &Config,
    consumer: &str,
    job: &Job,
) -> Result<()> {
    if registry::handles(job) {
        registry::prepare_completion(database, config, consumer, job).await?;
    }
    Ok(())
}

pub(crate) async fn record_completion(
    transaction: &DatabaseTransaction,
    job: &Job,
    status: &str,
    last_error: Option<String>,
) -> Result<()> {
    if registry::handles(job) {
        registry::record_completion(transaction, job, status, last_error).await?;
    }
    Ok(())
}

pub(crate) async fn complete(config: &Config, job: &Job) -> Result<()> {
    if registry::handles(job) {
        registry::complete(config).await?;
    }
    Ok(())
}
