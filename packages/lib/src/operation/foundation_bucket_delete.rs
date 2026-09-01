use aws_sdk_s3::config::BehaviorVersion;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Context, Operation, Result};
use crate::secrets::{self, PLATFORM_KEY};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FoundationBucketDelete {
    pub bucket_id: Uuid,
    pub provider_id: Uuid,
}

#[derive(Deserialize)]
struct ProviderSecret {
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
}

struct ProviderCredentials {
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
    endpoint_url: String,
    provider_region: Option<String>,
}

impl Operation<FoundationBucketDelete> {
    pub const QUEUE: &'static str = "foundation";
    pub const NAME: &'static str = "foundation_bucket_delete";

    pub async fn run(&self, context: &Context<'_>) -> Result<()> {
        let job = self;
        let credentials = provider_credentials(context, job.input.provider_id).await?;
        let region = credentials
            .provider_region
            .clone()
            .unwrap_or_else(|| "us-east-1".into());
        let client = aws_sdk_s3::Client::from_conf(
            aws_sdk_s3::Config::builder()
                .behavior_version(BehaviorVersion::latest())
                .endpoint_url(credentials.endpoint_url)
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
        crate::buckets::empty(&client, job.input.bucket_id).await?;
        crate::buckets::delete(&client, job.input.bucket_id).await
    }
}

async fn provider_credentials(
    context: &Context<'_>,
    provider_id: Uuid,
) -> Result<ProviderCredentials> {
    let row = context
        .database
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT s3_providers.endpoint_url, s3_providers.provider_region, secret.ciphertext FROM s3_providers JOIN secret ON secret.id=s3_providers.credential_secret_id WHERE s3_providers.id=$1::uuid AND s3_providers.is_active=true AND secret.scope='platform'::secret_scope AND secret.organization_id IS NULL",
            vec![provider_id.into()],
        ))
        .await?
        .ok_or_else(|| {
            Box::new(std::io::Error::other("S3 provider not found")) as super::Error
        })?;
    let ciphertext: String = row.try_get("", "ciphertext")?;
    let plaintext = secrets::decrypt(context.secrets, PLATFORM_KEY, &ciphertext).await?;
    let secret: ProviderSecret = serde_json::from_slice(&plaintext)?;
    Ok(ProviderCredentials {
        access_key_id: secret.access_key_id,
        secret_access_key: secret.secret_access_key,
        session_token: secret.session_token,
        endpoint_url: row.try_get("", "endpoint_url")?,
        provider_region: row.try_get("", "provider_region")?,
    })
}
