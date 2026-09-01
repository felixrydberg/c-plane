use lib::operation::{self, Context, Operation};
use sea_orm::{Database, DatabaseConnection};
use std::{env, time::Duration};
use tokio::{task::JoinSet, time::Instant};
use tracing::{error, info, warn};
use uuid::Uuid;

pub(crate) type Error = Box<dyn std::error::Error + Send + Sync>;
pub(crate) type Result<T> = std::result::Result<T, Error>;

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub(crate) struct Config {
    pub(crate) queues: String,
    pub(crate) concurrency: usize,
    pub(crate) secrets: lib::secrets::Client,
    pub(crate) registry_http: reqwest::Client,
    pub(crate) registry_internal_url: String,
    pub(crate) service_token: String,
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
        Ok(Self {
            queues: queues.join(","),
            concurrency,
            secrets: lib::secrets::Client::from_env()?,
            registry_http: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .build()?,
            registry_internal_url: required_env("REGISTRY_INTERNAL_URL")?
                .trim_end_matches('/')
                .to_owned(),
            service_token: required_env("CPLANE_SERVICE_TOKEN")?,
        })
    }
}

async fn consume(database: DatabaseConnection, config: Config, consumer: String) {
    loop {
        match operation::pull(&database, &config.queues, &consumer).await {
            Ok(Some(operation))
                if operation.metadata.attempts > operation.metadata.max_attempts =>
            {
                let job = &operation.metadata;
                let outcome = Err(format!("maximum attempts ({}) exceeded", job.max_attempts));
                if let Err(finish_error) =
                    operation::finish(&database, &consumer, &operation, &outcome).await
                {
                    error!(job_id = %job.id, %finish_error, "failed to exhaust job");
                } else if let Err(error) = outcome {
                    warn!(job_id = %job.id, %error, "job failed");
                }
            }
            Ok(Some(operation)) => {
                let job = &operation.metadata;
                info!(job_id = %job.id, queue = job.queue_name, job_type = job.job_type, attempt = job.attempts, "job claimed");
                if let Some(outcome) =
                    run_with_lease(&database, &config, &consumer, &operation).await
                {
                    if let Err(finish_error) =
                        operation::finish(&database, &consumer, &operation, &outcome).await
                    {
                        error!(job_id = %job.id, %finish_error, "failed to finish job");
                    } else {
                        match outcome {
                            Ok(()) => info!(job_id = %job.id, "job succeeded"),
                            Err(error) => warn!(job_id = %job.id, %error, "job failed"),
                        }
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

async fn run_with_lease(
    database: &DatabaseConnection,
    config: &Config,
    consumer: &str,
    operation: &Operation,
) -> Option<std::result::Result<(), String>> {
    let job = &operation.metadata;
    let context = Context {
        database,
        consumer,
        secrets: &config.secrets,
        registry_http: &config.registry_http,
        registry_internal_url: &config.registry_internal_url,
        service_token: &config.service_token,
    };
    let mut run = Box::pin(operation.run(&context));
    let mut heartbeat =
        tokio::time::interval_at(Instant::now() + HEARTBEAT_INTERVAL, HEARTBEAT_INTERVAL);
    loop {
        tokio::select! {
            result = &mut run => return Some(result.map_err(|error| error.to_string())),
            _ = heartbeat.tick() => match operation::renew(database, job.id, consumer).await {
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
