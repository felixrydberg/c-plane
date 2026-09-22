use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::entities::{bucket, bucket_grant, secret};
use crate::secrets;
use aws_sdk_s3::{
    error::ProvideErrorMetadata,
    types::{BucketLocationConstraint, CreateBucketConfiguration, Delete, ObjectIdentifier},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseTransaction,
    EntityTrait, QueryFilter, Set, Statement, Value,
};
use serde::Serialize;
use uuid::Uuid;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

const BUCKET_PREFIX: &str = "cp-";
const DELETE_BATCH_SIZE: i32 = 1_000;

#[derive(Debug, PartialEq, Eq)]
pub struct PrefixDeletion {
    pub deleted: usize,
    pub next_continuation_token: Option<String>,
}

#[derive(Debug)]
struct S3ProviderError {
    code: Option<String>,
    message: Option<String>,
}

impl std::fmt::Display for S3ProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.code, &self.message) {
            (Some(code), Some(message)) => write!(formatter, "{code}: {message}"),
            (Some(code), None) => formatter.write_str(code),
            (None, Some(message)) => formatter.write_str(message),
            (None, None) => formatter.write_str("S3 provider request failed"),
        }
    }
}

impl std::error::Error for S3ProviderError {}

fn s3_provider_error<E>(error: E) -> Error
where
    E: std::error::Error + Send + Sync + 'static + ProvideErrorMetadata,
{
    Box::new(S3ProviderError {
        code: error.code().map(str::to_owned),
        message: error.message().map(str::to_owned),
    })
}

pub fn error_details(error: &Error) -> (Option<&str>, Option<&str>) {
    error
        .as_ref()
        .downcast_ref::<S3ProviderError>()
        .map(|error| (error.code.as_deref(), error.message.as_deref()))
        .unwrap_or((None, None))
}

pub fn is_credentials_error(error: &Error) -> bool {
    error_details(error)
        .0
        .is_some_and(is_credentials_error_code)
}

fn is_credentials_error_code(code: &str) -> bool {
    matches!(
        code,
        "ExpiredToken"
            | "InvalidAccessKeyId"
            | "InvalidSecurity"
            | "InvalidToken"
            | "SignatureDoesNotMatch"
    )
}

pub async fn create(
    client: &aws_sdk_s3::Client,
    region: Option<&str>,
    bucket_id: Uuid,
) -> Result<()> {
    match client
        .head_bucket()
        .bucket(physical_bucket_name(bucket_id))
        .send()
        .await
    {
        Ok(_) => return Ok(()),
        Err(error)
            if error
                .raw_response()
                .is_some_and(|response| response.status().as_u16() == 404) => {}
        Err(error) => return Err(s3_provider_error(error)),
    }
    let mut request = client
        .create_bucket()
        .bucket(physical_bucket_name(bucket_id));
    if let Some(region) = region.filter(|region| *region != "us-east-1") {
        request = request.create_bucket_configuration(
            CreateBucketConfiguration::builder()
                .location_constraint(BucketLocationConstraint::from(region))
                .build(),
        );
    }
    if let Err(error) = request.send().await
        && error.as_service_error().and_then(|error| error.code())
            != Some("BucketAlreadyOwnedByYou")
    {
        return Err(s3_provider_error(error));
    }
    Ok(())
}

pub async fn create_foundation(
    connection: &DatabaseTransaction,
    secrets_client: &secrets::Client,
    organization_id: Uuid,
    region_id: Uuid,
    bucket_id: Uuid,
) -> Result<()> {
    if bucket::Entity::find_by_id(bucket_id)
        .one(connection)
        .await?
        .is_some()
    {
        return Ok(());
    }
    let secret_id = Uuid::new_v4();
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let mut key = [0_u8; 32];
    key[..16].copy_from_slice(first.as_bytes());
    key[16..].copy_from_slice(second.as_bytes());
    let tenant_key = format!("tenant-{}", organization_id.simple());
    let ciphertext =
        secrets::encrypt(secrets_client, &tenant_key, STANDARD.encode(key).as_bytes()).await?;

    secret::ActiveModel {
        id: Set(secret_id),
        scope: Set(secret::SecretScope::Tenant),
        organization_id: Set(Some(organization_id)),
        ciphertext: Set(ciphertext),
        ..Default::default()
    }
    .insert(connection)
    .await?;
    bucket::ActiveModel {
        id: Set(bucket_id),
        region_id: Set(region_id),
        sse_secret_id: Set(secret_id),
        status: Set(bucket::BucketStatus::Active),
        ..Default::default()
    }
    .insert(connection)
    .await?;
    Ok(())
}

pub fn physical_bucket_name(id: Uuid) -> String {
    format!("{BUCKET_PREFIX}{}", id.simple())
}

pub async fn is_empty(client: &aws_sdk_s3::Client, bucket_id: Uuid) -> Result<bool> {
    let output = match client
        .list_objects_v2()
        .bucket(physical_bucket_name(bucket_id))
        .max_keys(1)
        .send()
        .await
    {
        Ok(output) => output,
        Err(error)
            if error.as_service_error().and_then(|error| error.code()) == Some("NoSuchBucket") =>
        {
            return Ok(true);
        }
        Err(error) => return Err(Box::new(error)),
    };
    Ok(output.contents().is_empty())
}

pub async fn empty(client: &aws_sdk_s3::Client, bucket_id: Uuid) -> Result<()> {
    let bucket = physical_bucket_name(bucket_id);
    let mut continuation_token = None;

    loop {
        let output = match client
            .list_objects_v2()
            .bucket(&bucket)
            .set_continuation_token(continuation_token)
            .max_keys(DELETE_BATCH_SIZE)
            .send()
            .await
        {
            Ok(output) => output,
            Err(error)
                if error.as_service_error().and_then(|error| error.code())
                    == Some("NoSuchBucket") =>
            {
                return Ok(());
            }
            Err(error) => return Err(Box::new(error)),
        };
        let objects = output
            .contents()
            .iter()
            .filter_map(|object| object.key())
            .map(|key| ObjectIdentifier::builder().key(key).build())
            .collect::<std::result::Result<Vec<_>, _>>()?;

        if !objects.is_empty() {
            let delete = Delete::builder()
                .set_objects(Some(objects))
                .quiet(true)
                .build()?;
            let response = client
                .delete_objects()
                .bucket(&bucket)
                .delete(delete)
                .send()
                .await?;
            if !response.errors().is_empty() {
                return Err(Box::new(std::io::Error::other(format!(
                    "S3 provider returned {} object deletion errors",
                    response.errors().len()
                ))));
            }
            continuation_token = None;
            continue;
        }

        continuation_token = output.next_continuation_token().map(str::to_owned);
        if continuation_token.is_none() {
            return Ok(());
        }
    }
}

pub fn normalize_prefix(prefix: &str) -> String {
    if prefix.ends_with('/') {
        prefix.to_owned()
    } else {
        format!("{prefix}/")
    }
}

pub async fn delete_prefix(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    prefix: &str,
    continuation_token: Option<String>,
    max_pages: usize,
) -> Result<PrefixDeletion> {
    let prefix = normalize_prefix(prefix);
    let mut continuation_token = continuation_token;
    let mut deleted = 0;

    for _ in 0..max_pages {
        let output = match client
            .list_objects_v2()
            .bucket(bucket)
            .prefix(&prefix)
            .set_continuation_token(continuation_token)
            .send()
            .await
        {
            Ok(output) => output,
            Err(error)
                if error.as_service_error().and_then(|error| error.code())
                    == Some("NoSuchBucket") =>
            {
                return Ok(PrefixDeletion {
                    deleted,
                    next_continuation_token: None,
                });
            }
            Err(error) => return Err(Box::new(error)),
        };
        let identifiers = output
            .contents()
            .iter()
            .filter_map(|object| object.key())
            .map(|key| ObjectIdentifier::builder().key(key).build())
            .collect::<std::result::Result<Vec<_>, _>>()?;

        if !identifiers.is_empty() {
            let batch_size = identifiers.len();
            let delete = Delete::builder()
                .set_objects(Some(identifiers))
                .quiet(true)
                .build()?;
            let response = client
                .delete_objects()
                .bucket(bucket)
                .delete(delete)
                .send()
                .await?;
            if !response.errors().is_empty() {
                return Err(Box::new(std::io::Error::other(format!(
                    "S3 provider returned {} object deletion errors",
                    response.errors().len()
                ))));
            }
            deleted += batch_size;
        }

        continuation_token = output.next_continuation_token().map(str::to_owned);
        if continuation_token.is_none() {
            return Ok(PrefixDeletion {
                deleted,
                next_continuation_token: None,
            });
        }
    }

    Ok(PrefixDeletion {
        deleted,
        next_continuation_token: continuation_token,
    })
}

pub async fn delete_foundation(
    connection: &DatabaseTransaction,
    bucket_id: Uuid,
) -> std::result::Result<bool, sea_orm::DbErr> {
    let Some(foundation) = bucket::Entity::find_by_id(bucket_id)
        .one(connection)
        .await?
    else {
        return Ok(false);
    };

    bucket_grant::Entity::delete_many()
        .filter(bucket_grant::Column::BucketId.eq(bucket_id))
        .exec(connection)
        .await?;
    bucket::Entity::delete_by_id(bucket_id)
        .exec(connection)
        .await?;
    secret::Entity::delete_by_id(foundation.sse_secret_id)
        .exec(connection)
        .await?;
    Ok(true)
}

pub async fn delete(client: &aws_sdk_s3::Client, bucket_id: Uuid) -> Result<()> {
    if let Err(error) = client
        .delete_bucket()
        .bucket(physical_bucket_name(bucket_id))
        .send()
        .await
        && error.as_service_error().and_then(|error| error.code()) != Some("NoSuchBucket")
    {
        return Err(s3_provider_error(error));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_credentials_error_code, normalize_prefix, physical_bucket_name};
    use axum::{
        Router,
        http::{Method, StatusCode, Uri},
        routing::any,
    };
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    async fn s3_client(app: Router) -> (aws_sdk_s3::Client, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let config = aws_sdk_s3::config::Builder::new()
            .behavior_version_latest()
            .region(aws_sdk_s3::config::Region::new("us-east-1"))
            .endpoint_url(format!("http://{address}"))
            .credentials_provider(aws_sdk_s3::config::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .force_path_style(true)
            .build();
        (aws_sdk_s3::Client::from_conf(config), server)
    }

    #[tokio::test]
    async fn repeated_creation_reuses_the_physical_bucket() {
        let bucket_id = Uuid::new_v4();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();
        let app = Router::new().fallback(any(move |method: Method, uri: Uri| {
            assert_eq!(
                uri.path().trim_end_matches('/'),
                format!("/{}", physical_bucket_name(bucket_id))
            );
            let mut requests = captured.lock().unwrap();
            requests.push(method);
            let status = if requests.len() == 1 {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::OK
            };
            async move { status }
        }));
        let (client, server) = s3_client(app).await;
        super::create(&client, Some("us-east-1"), bucket_id)
            .await
            .unwrap();
        super::create(&client, Some("us-east-1"), bucket_id)
            .await
            .unwrap();
        server.abort();
        assert_eq!(
            *requests.lock().unwrap(),
            [Method::HEAD, Method::PUT, Method::HEAD]
        );
    }

    #[tokio::test]
    async fn denied_bucket_lookup_does_not_create_a_bucket() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();
        let app = Router::new().fallback(any(move |method: Method| {
            captured.lock().unwrap().push(method);
            async { StatusCode::FORBIDDEN }
        }));
        let (client, server) = s3_client(app).await;
        let result = super::create(&client, Some("us-east-1"), Uuid::new_v4()).await;
        server.abort();
        assert!(result.is_err());
        assert_eq!(*requests.lock().unwrap(), [Method::HEAD]);
    }

    #[test]
    fn normalizes_folder_prefixes() {
        assert_eq!(normalize_prefix("folder"), "folder/");
        assert_eq!(normalize_prefix("folder/"), "folder/");
    }

    #[test]
    fn physical_bucket_names_use_prefixed_hyphenless_ids() {
        let id = Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap();

        assert_eq!(
            physical_bucket_name(id),
            "cp-67e5504410b1426f9247bb680e5fe0c8"
        );
    }

    #[test]
    fn identifies_credential_error_codes() {
        assert!(is_credentials_error_code("InvalidAccessKeyId"));
        assert!(is_credentials_error_code("SignatureDoesNotMatch"));
        assert!(!is_credentials_error_code("AccessDenied"));
    }
}

pub mod credentials {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Credential {
        pub id: Uuid,
        pub secret_id: Uuid,
        pub organization_id: Option<Uuid>,
        pub name: String,
        pub prefix: String,
        pub ciphertext: String,
    }

    pub async fn create<T: Serialize>(
        client: &secrets::Client,
        organization_id: Option<Uuid>,
        name: impl Into<String>,
        prefix: impl Into<String>,
        value: &T,
    ) -> Result<Credential> {
        let plaintext = serde_json::to_vec(value)?;
        let key = transit_key(organization_id);
        let ciphertext = secrets::encrypt(client, &key, &plaintext).await?;
        Ok(Credential {
            id: Uuid::new_v4(),
            secret_id: Uuid::new_v4(),
            organization_id,
            name: name.into(),
            prefix: prefix.into(),
            ciphertext,
        })
    }

    pub async fn insert<C: ConnectionTrait>(connection: &C, credential: &Credential) -> Result<()> {
        let scope = if credential.organization_id.is_some() {
            "tenant"
        } else {
            "platform"
        };
        connection
            .execute(statement(
                "WITH inserted_secret AS (INSERT INTO secret (id, scope, organization_id, ciphertext) VALUES ($1, $2::secret_scope, $3, $4)) INSERT INTO credential (id, organization_id, access_key_id, secret_id, prefix) VALUES ($5, $3, $6, $1, $7)",
                vec![
                    credential.secret_id.into(),
                    scope.into(),
                    credential.organization_id.into(),
                    credential.ciphertext.clone().into(),
                    credential.id.into(),
                    credential.name.clone().into(),
                    credential.prefix.clone().into(),
                ],
            ))
            .await?;
        Ok(())
    }

    pub async fn delete<C: ConnectionTrait>(connection: &C, id: Uuid) -> Result<()> {
        let result = connection
            .execute(statement(
                "WITH deleted_credential AS (DELETE FROM credential WHERE id=$1 RETURNING secret_id) DELETE FROM secret USING deleted_credential WHERE secret.id=deleted_credential.secret_id",
                vec![id.into()],
            ))
            .await?;
        if result.rows_affected() == 0 {
            return Err(Box::new(sea_orm::DbErr::RecordNotFound(
                "Credential not found".to_owned(),
            )));
        }
        Ok(())
    }

    pub async fn roll<C: ConnectionTrait>(
        connection: &C,
        id: Uuid,
        ciphertext: &str,
    ) -> Result<()> {
        let result = connection
            .execute(statement(
                "UPDATE secret SET ciphertext=$2, updated_at=NOW() WHERE id=(SELECT secret_id FROM credential WHERE id=$1)",
                vec![id.into(), ciphertext.to_owned().into()],
            ))
            .await?;
        if result.rows_affected() == 0 {
            return Err(Box::new(sea_orm::DbErr::RecordNotFound(
                "Credential not found".to_owned(),
            )));
        }
        Ok(())
    }

    pub async fn rename<C: ConnectionTrait>(connection: &C, id: Uuid, name: &str) -> Result<()> {
        let result = connection
            .execute(statement(
                "UPDATE credential SET access_key_id=$2, updated_at=NOW() WHERE id=$1",
                vec![id.into(), name.to_owned().into()],
            ))
            .await?;
        if result.rows_affected() == 0 {
            return Err(Box::new(sea_orm::DbErr::RecordNotFound(
                "Credential not found".to_owned(),
            )));
        }
        Ok(())
    }

    fn transit_key(organization_id: Option<Uuid>) -> String {
        organization_id.map_or_else(
            || secrets::PLATFORM_KEY.to_owned(),
            |id| format!("tenant-{}", id.simple()),
        )
    }

    fn statement(sql: &str, values: Vec<Value>) -> Statement {
        Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use uuid::Uuid;

        #[test]
        fn transit_keys_are_scope_safe() {
            let organization_id = Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap();
            assert_eq!(transit_key(None), "platform");
            assert_eq!(
                transit_key(Some(organization_id)),
                "tenant-67e5504410b1426f9247bb680e5fe0c8"
            );
        }
    }
}
