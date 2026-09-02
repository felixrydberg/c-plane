use aws_sdk_s3::{
    Client as S3Client,
    config::{BehaviorVersion, Credentials, Region},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    buckets,
    entities::{bucket, bucket_grant, s3_provider, secret},
    secrets::{self, Client, PLATFORM_KEY},
};

const ACCESS_KEY_PREFIX: &str = "CP";

#[derive(Serialize)]
struct StorageSecret {
    secret_access_key: String,
}

pub struct ProvisionedPlatformBucket {
    pub bucket_id: Uuid,
    pub storage_credential: buckets::credentials::Credential,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub provider: S3Client,
}

pub async fn provision_platform_bucket<C: ConnectionTrait>(
    connection: &C,
    secrets_client: &Client,
    s3_provider_id: Uuid,
    supplied_credentials: Option<(String, String)>,
) -> Result<ProvisionedPlatformBucket, Box<dyn std::error::Error + Send + Sync>> {
    let provider = s3_provider::Entity::find_by_id(s3_provider_id)
        .one(connection)
        .await?
        .ok_or_else(|| format!("S3 provider {s3_provider_id} not found"))?;
    if !provider.is_active {
        return Err("S3 provider is inactive".into());
    }
    let provider_secret = secret::Entity::find_by_id(provider.credential_secret_id)
        .one(connection)
        .await?
        .ok_or_else(|| "S3 provider secret not found".to_owned())?;
    let provider_credentials: ProviderCredentials = serde_json::from_slice(
        &secrets::decrypt(secrets_client, PLATFORM_KEY, &provider_secret.ciphertext).await?,
    )?;
    let s3_client = S3Client::from_conf(
        aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url(provider.endpoint_url)
            .region(Region::new(provider.provider_region.clone()))
            .credentials_provider(Credentials::new(
                provider_credentials.access_key_id,
                provider_credentials.secret_access_key,
                provider_credentials.session_token,
                None,
                "c-plane-provisioner",
            ))
            .force_path_style(true)
            .build(),
    );

    let bucket_id = Uuid::new_v4();
    let sse_secret_id = Uuid::new_v4(); 
    let mut sse_key = [0_u8; 32];
    sse_key[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    sse_key[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    let sse_ciphertext = secrets::encrypt(
        secrets_client,
        PLATFORM_KEY,
        STANDARD.encode(sse_key).as_bytes(),
    )
    .await?;
    secret::ActiveModel {
        id: Set(sse_secret_id),
        scope: Set(secret::SecretScope::Platform),
        organization_id: Set(None),
        ciphertext: Set(sse_ciphertext),
        ..Default::default()
    }
    .insert(connection)
    .await?;
    bucket::ActiveModel {
        id: Set(bucket_id),
        s3_provider_id: Set(s3_provider_id),
        sse_secret_id: Set(sse_secret_id),
        status: Set(bucket::BucketStatus::Active),
        ..Default::default()
    }
    .insert(connection)
    .await?;

    let (access_key_id, secret_access_key) = supplied_credentials.unwrap_or_else(|| {
        (
            format!("{ACCESS_KEY_PREFIX}{}", Uuid::new_v4().simple()).to_uppercase(),
            format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()),
        )
    });
    if access_key_id.trim().is_empty() || secret_access_key.trim().is_empty() {
        return Err("storage credentials are required".into());
    }
    let storage_credential = buckets::credentials::create(
        secrets_client,
        None,
        access_key_id.clone(),
        &StorageSecret {
            secret_access_key: secret_access_key.clone(),
        },
    )
    .await?;
    buckets::credentials::insert(connection, &storage_credential).await?;
    bucket_grant::ActiveModel {
        id: Set(Uuid::new_v4()),
        credential_id: Set(storage_credential.id),
        bucket_id: Set(bucket_id),
        organization_id: Set(None),
        prefix: Set(String::new()),
        can_read: Set(true),
        can_write: Set(true),
        ..Default::default()
    }
    .insert(connection)
    .await?;

    if let Err(error) = buckets::create(
        &s3_client,
        Some(provider.provider_region.as_str()),
        bucket_id,
    )
    .await
    {
        let _ = buckets::delete(&s3_client, bucket_id).await;
        return Err(error);
    }
    Ok(ProvisionedPlatformBucket {
        bucket_id,
        storage_credential,
        access_key_id,
        secret_access_key,
        provider: s3_client,
    })
}

#[derive(Serialize, serde::Deserialize)]
pub struct ProviderCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}
