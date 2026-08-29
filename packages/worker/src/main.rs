use sea_orm::{
    ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, QueryResult, Statement,
    TransactionTrait, Value,
};
use std::{env, time::Duration};
use tokio::{task::JoinSet, time::Instant};
use tracing::{error, info, warn};
use uuid::Uuid;

mod services;

pub(crate) type Error = Box<dyn std::error::Error + Send + Sync>;
pub(crate) type Result<T> = std::result::Result<T, Error>;

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub(crate) struct Config {
    pub(crate) queues: String,
    pub(crate) concurrency: usize,
    pub(crate) redis_url: String,
    pub(crate) secrets: lib::secrets::Client,
    pub(crate) registry_token_ttl: Duration,
    pub(crate) registry_access_key: String,
    pub(crate) registry_gc_access_key: String,
}

#[derive(Clone)]
pub(crate) struct Job {
    pub(crate) id: Uuid,
    pub(crate) queue_name: String,
    pub(crate) job_type: String,
    pub(crate) attempts: i32,
    pub(crate) max_attempts: i32,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let config = Config::from_env()?;
    let database = Database::connect(required_env("DATABASE_URL")?).await?;
    let worker_id = Uuid::new_v4().to_string();
    let mut consumers = JoinSet::new();
    for slot in 0..config.concurrency {
        consumers.spawn(consume(
            database.clone(),
            config.clone(),
            format!("{worker_id}:{slot}"),
        ));
    }
    info!(
        queues = config.queues,
        concurrency = config.concurrency,
        "worker started"
    );

    tokio::select! {
        _ = tokio::signal::ctrl_c() => info!("worker stopping"),
        result = consumers.join_next() => {
            match result {
                Some(Ok(())) => return Err(message("worker consumer stopped unexpectedly")),
                Some(Err(error)) => return Err(Box::new(error)),
                None => return Err(message("worker started without consumers")),
            }
        }
    }
    consumers.abort_all();
    Ok(())
}

impl Config {
    fn from_env() -> Result<Self> {
        let queues = env::var("WORKER_QUEUES").unwrap_or_else(|_| "maintenance".into());
        let queues = queues
            .split(',')
            .map(str::trim)
            .filter(|queue| !queue.is_empty())
            .collect::<Vec<_>>();
        if queues.is_empty() || queues.iter().any(|queue| !valid_queue_name(queue)) {
            return Err(message("WORKER_QUEUES contains an invalid queue name"));
        }
        let concurrency = env::var("WORKER_CONCURRENCY")
            .unwrap_or_else(|_| "1".into())
            .parse::<usize>()
            .map_err(|_| message("WORKER_CONCURRENCY must be a number"))?;
        if !(1..=64).contains(&concurrency) {
            return Err(message("WORKER_CONCURRENCY must be between 1 and 64"));
        }
        let token_ttl_seconds = env::var("REGISTRY_TOKEN_TTL_SECONDS")
            .unwrap_or_else(|_| "60".into())
            .parse::<u64>()
            .map_err(|_| message("REGISTRY_TOKEN_TTL_SECONDS must be a number"))?;
        if token_ttl_seconds < 60 {
            return Err(message("REGISTRY_TOKEN_TTL_SECONDS must be at least 60"));
        }
        Ok(Self {
            queues: queues.join(","),
            concurrency,
            redis_url: required_env("REDIS_URL")?,
            secrets: lib::secrets::Client::from_env()?,
            registry_token_ttl: Duration::from_secs(token_ttl_seconds),
            registry_access_key: env::var("REGISTRY_STORAGE_S3_ACCESSKEY").unwrap_or_default(),
            registry_gc_access_key: env::var("REGISTRY_STORAGE_S3_GC_ACCESSKEY")
                .unwrap_or_default(),
        })
    }
}

async fn consume(database: DatabaseConnection, config: Config, consumer: String) {
    loop {
        match claim_job(&database, &config.queues, &consumer).await {
            Ok(Some(job)) if job.attempts > job.max_attempts => {
                let error = format!("maximum attempts ({}) exceeded", job.max_attempts);
                if let Err(finish_error) =
                    finish_job(&database, &config, &consumer, &job, Err(error)).await
                {
                    error!(job_id = %job.id, %finish_error, "failed to exhaust job");
                }
            }
            Ok(Some(job)) => {
                info!(job_id = %job.id, queue = job.queue_name, job_type = job.job_type, attempt = job.attempts, "job claimed");
                if let Some(result) = run_with_lease(&database, &config, &consumer, &job).await {
                    if let Err(finish_error) =
                        finish_job(&database, &config, &consumer, &job, result).await
                    {
                        error!(job_id = %job.id, %finish_error, "failed to finish job");
                    }
                } else {
                    warn!(job_id = %job.id, "job lease lost; handler cancelled");
                }
            }
            Ok(None) => tokio::time::sleep(POLL_INTERVAL).await,
            Err(claim_error) => {
                error!(%claim_error, "failed to claim job");
                tokio::time::sleep(POLL_INTERVAL).await;
            }
        }
    }
}

async fn claim_job(
    database: &DatabaseConnection,
    queues: &str,
    consumer: &str,
) -> Result<Option<Job>> {
    let row = database
        .query_one(statement(
            "WITH candidate AS (SELECT id FROM worker_queue WHERE queue_name=ANY(string_to_array($1, ',')) AND ((status='queued' AND available_at<=NOW()) OR (status='running' AND lease_expires_at<NOW())) ORDER BY created_at, id FOR UPDATE SKIP LOCKED LIMIT 1) UPDATE worker_queue job SET status='running', attempts=job.attempts+1, locked_by=$2, lease_expires_at=NOW()+INTERVAL '30 seconds', started_at=COALESCE(job.started_at, NOW()), updated_at=NOW() FROM candidate WHERE job.id=candidate.id RETURNING job.id, job.queue_name, job.job_type, job.attempts, job.max_attempts",
            vec![queues.to_owned().into(), consumer.to_owned().into()],
        ))
        .await?;
    row.map(job_from_row).transpose()
}

fn job_from_row(row: QueryResult) -> Result<Job> {
    Ok(Job {
        id: row.try_get("", "id")?,
        queue_name: row.try_get("", "queue_name")?,
        job_type: row.try_get("", "job_type")?,
        attempts: row.try_get("", "attempts")?,
        max_attempts: row.try_get("", "max_attempts")?,
    })
}

async fn run_with_lease(
    database: &DatabaseConnection,
    config: &Config,
    consumer: &str,
    job: &Job,
) -> Option<std::result::Result<(), String>> {
    let mut handler = Box::pin(services::run(database, config, consumer, job));
    let mut heartbeat =
        tokio::time::interval_at(Instant::now() + HEARTBEAT_INTERVAL, HEARTBEAT_INTERVAL);
    loop {
        tokio::select! {
            result = &mut handler => return Some(result.map_err(|error| error.to_string())),
            _ = heartbeat.tick() => match renew_lease(database, job.id, consumer).await {
                Ok(true) => {},
                Ok(false) => return None,
                Err(error) => {
                    error!(job_id = %job.id, %error, "lease heartbeat failed");
                    return None;
                }
            }
        }
    }
}

async fn renew_lease(database: &DatabaseConnection, job_id: Uuid, consumer: &str) -> Result<bool> {
    let result = database
        .execute(statement(
            "UPDATE worker_queue SET lease_expires_at=NOW()+INTERVAL '30 seconds', updated_at=NOW() WHERE id=$1::uuid AND status='running' AND locked_by=$2",
            vec![job_id.into(), consumer.to_owned().into()],
        ))
        .await?;
    Ok(result.rows_affected() == 1)
}

async fn finish_job(
    database: &DatabaseConnection,
    config: &Config,
    consumer: &str,
    job: &Job,
    result: std::result::Result<(), String>,
) -> Result<()> {
    let (status, last_error) = match &result {
        Ok(()) => ("succeeded", None),
        Err(error) => ("failed", Some(error.clone())),
    };
    services::prepare_completion(database, config, consumer, job).await?;

    let transaction = database.begin().await?;
    let updated = transaction
        .execute(statement(
            "UPDATE worker_queue SET status=$3, last_error=$4, locked_by=NULL, lease_expires_at=NULL, finished_at=NOW(), updated_at=NOW() WHERE id=$1::uuid AND status='running' AND locked_by=$2",
            vec![job.id.into(), consumer.to_owned().into(), status.into(), last_error.clone().into()],
        ))
        .await?;
    if updated.rows_affected() != 1 {
        return Err(message("job lease was lost before completion"));
    }
    services::record_completion(&transaction, job, status, last_error).await?;
    transaction.commit().await?;
    services::complete(config, job).await?;
    match result {
        Ok(()) => info!(job_id = %job.id, "job succeeded"),
        Err(error) => warn!(job_id = %job.id, %error, "job failed"),
    }
    Ok(())
}

pub(crate) fn statement(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| message(format!("{name} is required")))
}

fn valid_queue_name(queue: &str) -> bool {
    !queue.is_empty()
        && queue
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

pub(crate) fn message(message: impl Into<String>) -> Error {
    Box::new(std::io::Error::other(message.into()))
}

#[cfg(test)]
mod tests {
    use super::valid_queue_name;

    #[test]
    fn validates_named_worker_queues() {
        assert!(valid_queue_name("maintenance"));
        assert!(valid_queue_name("cluster-state"));
        assert!(!valid_queue_name("cluster_state"));
    }
}
