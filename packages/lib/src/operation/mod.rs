use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, DatabaseTransaction, DbErr, Statement,
    TransactionTrait,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::secrets;

pub mod foundation_bucket_delete;
pub mod registry_gc;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub queue_name: String,
    pub job_type: String,
    pub attempts: i32,
    pub max_attempts: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation<T = JsonValue> {
    pub metadata: Metadata,
    pub input: T,
}

pub struct Context<'a> {
    pub database: &'a DatabaseConnection,
    pub consumer: &'a str,
    pub secrets: &'a secrets::Client,
    pub registry_http: &'a reqwest::Client,
    pub registry_internal_url: &'a str,
    pub service_token: &'a str,
}

impl<T: Serialize> Operation<T> {
    async fn insert(
        transaction: &DatabaseTransaction,
        organization_id: Option<Uuid>,
        queue_name: &str,
        job_type: &str,
        dedupe_key: Option<String>,
        input: T,
    ) -> std::result::Result<Self, DbErr> {
        let id = Uuid::new_v4();
        let payload =
            serde_json::to_value(&input).map_err(|error| DbErr::Custom(error.to_string()))?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "INSERT INTO worker_queue (id, organization_id, queue_name, job_type, dedupe_key, payload) VALUES ($1, $2, $3, $4, $5, $6)",
                vec![
                    id.into(),
                    organization_id.into(),
                    queue_name.to_owned().into(),
                    job_type.to_owned().into(),
                    dedupe_key.into(),
                    payload.into(),
                ],
            ))
            .await?;
        Ok(Self {
            metadata: Metadata {
                id,
                organization_id,
                queue_name: queue_name.into(),
                job_type: job_type.into(),
                attempts: 0,
                max_attempts: 3,
            },
            input,
        })
    }
}

impl<T: DeserializeOwned> Operation<T> {
    fn from(operation: &Operation) -> std::result::Result<Self, serde_json::Error> {
        Ok(Self {
            metadata: operation.metadata.clone(),
            input: serde_json::from_value(operation.input.clone())?,
        })
    }
}

impl Operation<JsonValue> {
    pub async fn run(&self, context: &Context<'_>) -> Result<()> {
        let queue = self.metadata.queue_name.as_str();
        let kind = self.metadata.job_type.as_str();
        if queue == Operation::<registry_gc::RegistryGc>::QUEUE
            && kind == Operation::<registry_gc::RegistryGc>::NAME
        {
            return Operation::<registry_gc::RegistryGc>::from(self)?
                .run(context)
                .await;
        }
        if queue == Operation::<foundation_bucket_delete::FoundationBucketDelete>::QUEUE
            && kind == Operation::<foundation_bucket_delete::FoundationBucketDelete>::NAME
        {
            return Operation::<foundation_bucket_delete::FoundationBucketDelete>::from(self)?
                .run(context)
                .await;
        }
        Err(Box::new(std::io::Error::other(format!(
            "unsupported operation: {queue}/{kind}"
        ))))
    }

    async fn complete(&self, transaction: &DatabaseTransaction) -> std::result::Result<(), DbErr> {
        if self.metadata.queue_name == Operation::<registry_gc::RegistryGc>::QUEUE
            && self.metadata.job_type == Operation::<registry_gc::RegistryGc>::NAME
            && let Ok(operation) = Operation::<registry_gc::RegistryGc>::from(self)
        {
            return operation.complete(transaction).await;
        }
        Ok(())
    }
}

pub async fn pull(
    database: &DatabaseConnection,
    queues: &str,
    consumer: &str,
) -> std::result::Result<Option<Operation>, DbErr> {
    let row = database
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "WITH candidate AS (SELECT id FROM worker_queue WHERE queue_name=ANY(string_to_array($1, ',')) AND ((status='queued' AND available_at<=NOW()) OR (status='running' AND lease_expires_at<NOW())) ORDER BY created_at, id FOR UPDATE SKIP LOCKED LIMIT 1) UPDATE worker_queue job SET status='running', attempts=job.attempts+1, locked_by=$2, lease_expires_at=NOW()+INTERVAL '30 seconds', started_at=COALESCE(job.started_at, NOW()), updated_at=NOW() FROM candidate WHERE job.id=candidate.id RETURNING job.id, job.organization_id, job.queue_name, job.job_type, job.payload, job.attempts, job.max_attempts",
            vec![queues.to_owned().into(), consumer.to_owned().into()],
        ))
        .await?;
    row.map(|row| {
        Ok(Operation {
            metadata: Metadata {
                id: row.try_get("", "id")?,
                organization_id: row.try_get("", "organization_id")?,
                queue_name: row.try_get("", "queue_name")?,
                job_type: row.try_get("", "job_type")?,
                attempts: row.try_get("", "attempts")?,
                max_attempts: row.try_get("", "max_attempts")?,
            },
            input: row.try_get("", "payload")?,
        })
    })
    .transpose()
}

pub async fn renew(
    database: &DatabaseConnection,
    id: Uuid,
    consumer: &str,
) -> std::result::Result<bool, DbErr> {
    let result = database
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE worker_queue SET lease_expires_at=NOW()+INTERVAL '30 seconds', updated_at=NOW() WHERE id=$1::uuid AND status='running' AND locked_by=$2",
            vec![id.into(), consumer.to_owned().into()],
        ))
        .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn finish(
    database: &DatabaseConnection,
    consumer: &str,
    operation: &Operation,
    outcome: &std::result::Result<(), String>,
) -> std::result::Result<(), DbErr> {
    let (status, error) = match outcome {
        Ok(()) => ("succeeded", None),
        Err(error) => ("failed", Some(error.clone())),
    };
    let transaction = database.begin().await?;
    let updated = transaction
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE worker_queue SET status=$3, last_error=$4, locked_by=NULL, lease_expires_at=NULL, finished_at=NOW(), updated_at=NOW() WHERE id=$1::uuid AND status='running' AND locked_by=$2",
            vec![
                operation.metadata.id.into(),
                consumer.to_owned().into(),
                status.into(),
                error.into(),
            ],
        ))
        .await?;
    if updated.rows_affected() != 1 {
        return Err(DbErr::Custom("job lease was lost before completion".into()));
    }
    operation.complete(&transaction).await?;
    transaction.commit().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn operation(payload: JsonValue) -> Operation {
        Operation {
            metadata: Metadata {
                id: Uuid::nil(),
                organization_id: Some(Uuid::nil()),
                queue_name: "maintenance".into(),
                job_type: "registry_gc".into(),
                attempts: 1,
                max_attempts: 3,
            },
            input: payload,
        }
    }

    #[test]
    fn creates_typed_registry_gc_operation() {
        assert!(
            Operation::<registry_gc::RegistryGc>::from(&operation(json!({
                "trigger": "manual"
            })))
            .is_ok()
        );
    }

    #[test]
    fn rejects_malformed_registry_gc_operation() {
        assert!(
            Operation::<registry_gc::RegistryGc>::from(&operation(json!({
                "unexpected": true
            })))
            .is_err()
        );
    }
}
