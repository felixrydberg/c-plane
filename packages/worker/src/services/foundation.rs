use crate::{Config, Job, Result, message, statement};
use lib::secrets::{self, PLATFORM_KEY};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
struct Payload {
    bucket_id: Uuid,
    provider_id: Uuid,
}

#[derive(Deserialize)]
struct ProviderSecret {
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
}

pub(super) async fn run(database: &DatabaseConnection, config: &Config, job: &Job) -> Result<()> {
    let row = database
        .query_one(statement(
            "SELECT payload FROM worker_queue WHERE id=$1::uuid",
            vec![job.id.into()],
        ))
        .await?
        .ok_or_else(|| message("foundation bucket delete job vanished"))?;
    let payload: Payload = serde_json::from_value(row.try_get("", "payload")?)?;
    let credentials = provider_credentials(database, &config.secrets, payload.provider_id).await?;
    let region = credentials
        .provider_region
        .clone()
        .unwrap_or_else(|| "us-east-1".into());
    let client = aws_sdk_s3::Client::from_conf(
        aws_sdk_s3::Config::builder()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .endpoint_url(credentials.endpoint_url.clone())
            .region(aws_sdk_s3::config::Region::new(region))
            .credentials_provider(aws_sdk_s3::config::Credentials::new(
                credentials.access_key_id,
                credentials.secret_access_key,
                credentials.session_token,
                None,
                "c-plane-worker",
            ))
            .force_path_style(true)
            .build(),
    );
    lib::buckets::empty(&client, payload.bucket_id).await?;
    lib::buckets::delete(&client, payload.bucket_id).await?;
    Ok(())
}

struct ProviderCredentials {
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
    endpoint_url: String,
    provider_region: Option<String>,
}

async fn provider_credentials(
    database: &DatabaseConnection,
    secrets: &secrets::Client,
    provider_id: Uuid,
) -> Result<ProviderCredentials> {
    let row = database
        .query_one(statement(
            "SELECT s3_providers.endpoint_url, s3_providers.provider_region, secret.ciphertext FROM s3_providers JOIN secret ON secret.id=s3_providers.credential_secret_id WHERE s3_providers.id=$1::uuid AND s3_providers.is_active=true",
            vec![provider_id.into()],
        ))
        .await?
        .ok_or_else(|| message("S3 provider not found"))?;
    let ciphertext: String = row.try_get("", "ciphertext")?;
    let plaintext = secrets::decrypt(secrets, PLATFORM_KEY, &ciphertext).await?;
    let secret: ProviderSecret = serde_json::from_slice(&plaintext)?;
    Ok(ProviderCredentials {
        access_key_id: secret.access_key_id,
        secret_access_key: secret.secret_access_key,
        session_token: secret.session_token,
        endpoint_url: row.try_get("", "endpoint_url")?,
        provider_region: row.try_get("", "provider_region")?,
    })
}
