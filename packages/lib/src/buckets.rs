use crate::secrets;
use aws_sdk_s3::{
    error::ProvideErrorMetadata,
    types::{BucketLocationConstraint, CreateBucketConfiguration, Delete, ObjectIdentifier},
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, Value};
use serde::Serialize;
use uuid::Uuid;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

const BUCKET_PREFIX: &str = "cp-";
const DELETE_BATCH_SIZE: i32 = 1_000;

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
    let request = client
        .create_bucket()
        .bucket(physical_bucket_name(bucket_id));
    if let Some(region) = region.filter(|region| *region != "us-east-1") {
        request
            .create_bucket_configuration(
                CreateBucketConfiguration::builder()
                    .location_constraint(BucketLocationConstraint::from(region))
                    .build(),
            )
            .send()
            .await
            .map_err(s3_provider_error)?;
    } else {
        request.send().await.map_err(s3_provider_error)?;
    }
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
    use super::{is_credentials_error_code, physical_bucket_name};
    use uuid::Uuid;

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
        pub ciphertext: String,
    }

    pub async fn create<T: Serialize>(
        client: &secrets::Client,
        organization_id: Option<Uuid>,
        name: impl Into<String>,
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
                "WITH inserted_secret AS (INSERT INTO secret (id, scope, organization_id, ciphertext) VALUES ($1, $2::secret_scope, $3, $4)) INSERT INTO credential (id, organization_id, access_key_id, secret_id) VALUES ($5, $3, $6, $1)",
                vec![
                    credential.secret_id.into(),
                    scope.into(),
                    credential.organization_id.into(),
                    credential.ciphertext.clone().into(),
                    credential.id.into(),
                    credential.name.clone().into(),
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
