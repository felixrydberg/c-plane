use crate::{Config, Job, Result, message, statement};
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction};
use std::{process::Stdio, time::Duration};
use tokio::process::Command;
use tracing::warn;
use uuid::Uuid;

pub(super) fn handles(job: &Job) -> bool {
    job.job_type == "registry_gc"
}

pub(super) async fn run(
    database: &DatabaseConnection,
    config: &Config,
    consumer: &str,
    job: &Job,
) -> Result<()> {
    set_phase(database, job.id, consumer, "draining").await?;
    invalidate_access_token_caches(config).await?;
    tokio::time::sleep(config.registry_token_ttl).await;
    set_phase(database, job.id, consumer, "collecting").await?;
    invalidate_access_token_caches(config).await?;

    let mut command = Command::new("/usr/local/bin/registry");
    command
        .args(["garbage-collect", "/etc/distribution/config.yml"])
        .env_remove("REGISTRY_TOKEN_TTL_SECONDS")
        .kill_on_drop(true)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = command.status().await?;
    if !status.success() {
        return Err(message(format!(
            "registry garbage collector exited with {status}"
        )));
    }
    Ok(())
}

pub(super) async fn prepare_completion(
    database: &DatabaseConnection,
    config: &Config,
    consumer: &str,
    job: &Job,
) -> Result<()> {
    set_phase(database, job.id, consumer, "restoring").await?;
    invalidate_access_token_caches(config).await
}

pub(super) async fn record_completion(
    transaction: &DatabaseTransaction,
    job: &Job,
    status: &str,
    last_error: Option<String>,
) -> Result<()> {
    transaction
        .execute(statement(
            "UPDATE registry_maintenance SET phase='idle', active_job_id=NULL, finished_at=NOW(), last_result=$2, last_error=$3, updated_at=NOW() WHERE service='distribution' AND active_job_id=$1::uuid",
            vec![job.id.into(), status.into(), last_error.into()],
        ))
        .await?;
    Ok(())
}

pub(super) async fn complete(config: &Config) -> Result<()> {
    invalidate_access_token_caches(config).await
}

async fn set_phase(
    database: &DatabaseConnection,
    job_id: Uuid,
    consumer: &str,
    phase: &str,
) -> Result<()> {
    let result = database
        .execute(statement(
            "UPDATE registry_maintenance maintenance SET phase=$3, updated_at=NOW() WHERE service='distribution' AND active_job_id=$1::uuid AND EXISTS (SELECT 1 FROM worker_queue job WHERE job.id=$1::uuid AND job.status='running' AND job.locked_by=$2 AND job.lease_expires_at>NOW())",
            vec![job_id.into(), consumer.to_owned().into(), phase.to_owned().into()],
        ))
        .await?;
    if result.rows_affected() != 1 {
        return Err(message("registry maintenance lease is no longer owned"));
    }
    Ok(())
}

async fn invalidate_access_token_caches(config: &Config) -> Result<()> {
    let access_keys = [
        config.registry_access_key.as_str(),
        config.registry_gc_access_key.as_str(),
    ];
    for attempt in 1..=5 {
        let result = async {
            let client = redis::Client::open(config.redis_url.as_str())?;
            let mut connection = client.get_multiplexed_async_connection().await?;
            for access_key in access_keys
                .iter()
                .filter(|access_key| !access_key.is_empty())
            {
                let _: u64 = redis::cmd("DEL")
                    .arg(format!(
                        "{}{}",
                        lib::cache::S3_ACCESS_TOKEN_CACHE_PREFIX,
                        access_key
                    ))
                    .query_async(&mut connection)
                    .await?;
            }
            Ok::<(), redis::RedisError>(())
        }
        .await;
        match result {
            Ok(()) => return Ok(()),
            Err(error) if attempt == 5 => return Err(Box::new(error)),
            Err(error) => {
                warn!(attempt, %error, "cache invalidation failed; retrying");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
    unreachable!()
}
