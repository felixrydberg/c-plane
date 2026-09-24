use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, JoinType, QueryFilter,
    QueryOrder, QuerySelect, RelationTrait, Set,
};
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    entities::{bucket_grant, credential, secret, storage, storage_access_token},
    error::AppError,
    secrets::{self, Client},
    services::s3_providers::S3ProviderClient,
    services::{buckets::tenant_key, events},
    tenant::TenantDatabase,
};

const ACCESS_KEY_PREFIX: &str = "CP";

#[derive(Clone)]
pub struct BucketPermission {
    pub bucket_id: Uuid,
    pub can_read: bool,
    pub can_write: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct AccessToken {
    pub id: Uuid,
    pub name: String,
    pub access_key_id: String,
    pub prefix: String,
    pub created_at: String,
}

pub struct CreatedAccessToken {
    pub token: AccessToken,
    pub secret_access_key: String,
}

pub struct AccessTokenDetails {
    pub token: AccessToken,
    pub bucket_permissions: Vec<BucketPermission>,
}

pub struct UpdateResult {
    pub access_key_id: String,
}

pub async fn create(
    tenant_db: &TenantDatabase,
    secrets_client: &Client,
    organization_id: Uuid,
    project_id: Uuid,
    name: &str,
    prefix: &str,
    permissions: &[BucketPermission],
    actor_id: Uuid,
) -> Result<CreatedAccessToken, AppError> {
    verify_org_owner(tenant_db, organization_id)?;
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::Conflict(
            "Token name must be 1-100 characters".into(),
        ));
    }
    if prefix.len() > 1024 {
        return Err(AppError::Conflict(
            "Credential prefix must be at most 1024 bytes".into(),
        ));
    }
    validate_permissions(permissions)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project(tx, organization_id, project_id).await?;
    let bucket_ids =
        verify_bucket_permissions(tx, organization_id, project_id, permissions).await?;
    if active_token_named(tx, project_id, name).await? {
        return Err(AppError::Conflict(
            "A token with this name already exists".into(),
        ));
    }
    let credential_id = Uuid::new_v4();
    let secret_id = Uuid::new_v4();
    let access_key_id = format!("{ACCESS_KEY_PREFIX}{}", Uuid::new_v4().simple()).to_uppercase();
    let secret_access_key = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let plaintext =
        serde_json::to_vec(&serde_json::json!({"secret_access_key": secret_access_key}))
            .map_err(|e| AppError::Internal(e.to_string()))?;
    let ciphertext =
        secrets::encrypt(secrets_client, &tenant_key(organization_id), &plaintext).await?;
    secret::ActiveModel {
        id: Set(secret_id),
        scope: Set(secret::SecretScope::Tenant),
        organization_id: Set(Some(organization_id)),
        project_id: Set(Some(project_id)),
        ciphertext: Set(ciphertext),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    credential::ActiveModel {
        id: Set(credential_id),
        organization_id: Set(Some(organization_id)),
        project_id: Set(Some(project_id)),
        access_key_id: Set(access_key_id),
        prefix: Set(prefix.into()),
        secret_id: Set(secret_id),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    storage_access_token::ActiveModel {
        credential_id: Set(credential_id),
        organization_id: Set(organization_id),
        project_id: Set(project_id),
        name: Set(name.into()),
    }
    .insert(tx)
    .await?;
    insert_grants(tx, credential_id, organization_id, &bucket_ids, permissions).await?;
    let token = token_by_id(tx, organization_id, project_id, credential_id).await?;
    events::record(tx, organization_id, project_id, "storage-access-token:created", serde_json::json!({"summary": format!("Created storage access token '{name}'"), "target_id": credential_id.to_string(), "bucket_ids": permissions.iter().map(|p| p.bucket_id).collect::<Vec<_>>() }), actor_id).await?;
    scoped.commit().await?;
    Ok(CreatedAccessToken {
        token,
        secret_access_key,
    })
}

pub async fn list(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<Vec<AccessToken>, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project(tx, organization_id, project_id).await?;
    let rows = storage_access_token::Entity::find()
        .find_also_related(credential::Entity)
        .filter(storage_access_token::Column::OrganizationId.eq(organization_id))
        .filter(storage_access_token::Column::ProjectId.eq(project_id))
        .filter(credential::Column::RevokedAt.is_null())
        .order_by_desc(credential::Column::CreatedAt)
        .all(tx)
        .await?;
    let result = rows
        .into_iter()
        .filter_map(|(token, c)| {
            c.map(|c| AccessToken {
                id: token.credential_id,
                name: token.name,
                access_key_id: c.access_key_id,
                prefix: c.prefix,
                created_at: c.created_at.to_string(),
            })
        })
        .collect();
    scoped.commit().await?;
    Ok(result)
}

pub async fn get(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
    token_id: Uuid,
) -> Result<AccessTokenDetails, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project(tx, organization_id, project_id).await?;
    let token = token_by_id(tx, organization_id, project_id, token_id).await?;
    let grants = bucket_grant::Entity::find()
        .filter(bucket_grant::Column::CredentialId.eq(token_id))
        .all(tx)
        .await?;
    let ids = grants.iter().map(|g| g.bucket_id).collect::<Vec<_>>();
    let mut buckets = storage::Entity::find()
        .filter(storage::Column::BucketId.is_in(ids))
        .all(tx)
        .await?;
    buckets.sort_by(|a, b| a.name.cmp(&b.name));
    let permissions = grants
        .into_iter()
        .map(|g| (g.bucket_id, (g.can_read, g.can_write)))
        .collect::<HashMap<_, _>>();
    let bucket_permissions = buckets
        .into_iter()
        .map(|b| {
            let (r, w) = permissions[&b.bucket_id];
            BucketPermission {
                bucket_id: b.id,
                can_read: r,
                can_write: w,
            }
        })
        .collect();
    scoped.commit().await?;
    Ok(AccessTokenDetails {
        token,
        bucket_permissions,
    })
}

pub async fn update(
    tenant_db: &TenantDatabase,
    providers: &S3ProviderClient,
    organization_id: Uuid,
    project_id: Uuid,
    token_id: Uuid,
    permissions: &[BucketPermission],
    actor_id: Uuid,
) -> Result<UpdateResult, AppError> {
    verify_org_owner(tenant_db, organization_id)?;
    validate_permissions(permissions)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project(tx, organization_id, project_id).await?;
    let token = token_by_id(tx, organization_id, project_id, token_id).await?;
    let ids = verify_bucket_permissions(tx, organization_id, project_id, permissions).await?;
    bucket_grant::Entity::delete_many()
        .filter(bucket_grant::Column::CredentialId.eq(token_id))
        .exec(tx)
        .await?;
    insert_grants(tx, token_id, organization_id, &ids, permissions).await?;
    events::record(tx,organization_id,project_id,"storage-access-token:updated",serde_json::json!({"summary":"Updated storage access token permissions","target_id":token_id.to_string(),"bucket_ids":permissions.iter().map(|p|p.bucket_id).collect::<Vec<_>>()}),actor_id).await?;
    scoped.commit().await?;
    if let Err(error) = providers
        .invalidate_access_token_cache(&token.access_key_id)
        .await
    {
        tracing::warn!(%error, %token_id, "access token cache invalidation failed after update");
    }
    Ok(UpdateResult {
        access_key_id: token.access_key_id,
    })
}

pub async fn revoke(
    tenant_db: &TenantDatabase,
    providers: &S3ProviderClient,
    organization_id: Uuid,
    project_id: Uuid,
    token_id: Uuid,
    actor_id: Uuid,
) -> Result<UpdateResult, AppError> {
    verify_org_owner(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project(tx, organization_id, project_id).await?;
    let token = token_by_id(tx, organization_id, project_id, token_id).await?;
    credential::ActiveModel {
        id: Set(token_id),
        revoked_at: Set(Some(Utc::now().into())),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    }
    .update(tx)
    .await?;
    events::record(tx,organization_id,project_id,"storage-access-token:revoked",serde_json::json!({"summary":format!("Revoked storage access token '{}'",token.name),"target_id":token_id.to_string()}),actor_id).await?;
    scoped.commit().await?;
    if let Err(error) = providers
        .invalidate_access_token_cache(&token.access_key_id)
        .await
    {
        tracing::warn!(%error, %token_id, "access token cache invalidation failed after revoke");
    }
    Ok(UpdateResult {
        access_key_id: token.access_key_id,
    })
}

pub async fn delete_for_project(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<Vec<String>, AppError> {
    let ids = storage_access_token::Entity::find()
        .filter(storage_access_token::Column::OrganizationId.eq(organization_id))
        .filter(storage_access_token::Column::ProjectId.eq(project_id))
        .select_only()
        .column(storage_access_token::Column::CredentialId)
        .into_tuple::<Uuid>()
        .all(tx)
        .await?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let keys = credential::Entity::find()
        .filter(credential::Column::Id.is_in(ids.clone()))
        .select_only()
        .column(credential::Column::AccessKeyId)
        .into_tuple::<String>()
        .all(tx)
        .await?;
    let secret_ids = credential::Entity::find()
        .filter(credential::Column::Id.is_in(ids.clone()))
        .select_only()
        .column(credential::Column::SecretId)
        .into_tuple::<Uuid>()
        .all(tx)
        .await?;
    bucket_grant::Entity::delete_many()
        .filter(bucket_grant::Column::CredentialId.is_in(ids.clone()))
        .exec(tx)
        .await?;
    storage_access_token::Entity::delete_many()
        .filter(storage_access_token::Column::OrganizationId.eq(organization_id))
        .filter(storage_access_token::Column::ProjectId.eq(project_id))
        .exec(tx)
        .await?;
    credential::Entity::delete_many()
        .filter(credential::Column::Id.is_in(ids))
        .exec(tx)
        .await?;
    secret::Entity::delete_many()
        .filter(secret::Column::Id.is_in(secret_ids))
        .exec(tx)
        .await?;
    Ok(keys)
}

async fn verify_project(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<(), AppError> {
    crate::entities::project::Entity::find_by_id(project_id)
        .filter(crate::entities::project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".into()))
        .map(|_| ())
}

fn verify_org_access(tenant_db: &TenantDatabase, organization_id: Uuid) -> Result<(), AppError> {
    if tenant_db
        .context
        .allowed_organizations
        .contains(&organization_id)
    {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "You do not have access to this organization".into(),
        ))
    }
}

fn verify_org_owner(tenant_db: &TenantDatabase, organization_id: Uuid) -> Result<(), AppError> {
    verify_org_access(tenant_db, organization_id)?;
    if let Some(api_key_organization_id) = tenant_db.context.api_key_organization_id {
        return if api_key_organization_id == organization_id {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "API key is not owned by this organization".into(),
            ))
        };
    }
    match tenant_db
        .context
        .organization_roles
        .get(&organization_id)
        .map(String::as_str)
    {
        Some("owner") => Ok(()),
        _ => Err(AppError::Forbidden(
            "Organization owner role required".into(),
        )),
    }
}
pub fn validate_permissions(permissions: &[BucketPermission]) -> Result<(), AppError> {
    if permissions.is_empty()
        || permissions.iter().any(|p| !p.can_read && !p.can_write)
        || permissions
            .iter()
            .map(|p| p.bucket_id)
            .collect::<std::collections::HashSet<_>>()
            .len()
            != permissions.len()
    {
        return Err(AppError::Conflict("Invalid bucket permissions".into()));
    }
    Ok(())
}
async fn verify_bucket_permissions(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
    permissions: &[BucketPermission],
) -> Result<Vec<Uuid>, AppError> {
    let ids = permissions.iter().map(|p| p.bucket_id).collect::<Vec<_>>();
    let buckets = storage::Entity::find()
        .filter(storage::Column::Id.is_in(ids))
        .filter(storage::Column::ProjectId.eq(project_id))
        .filter(storage::Column::OrganizationId.eq(organization_id))
        .all(tx)
        .await?
        .into_iter()
        .map(|b| (b.id, b.bucket_id))
        .collect::<HashMap<_, _>>();
    permissions
        .iter()
        .map(|p| {
            buckets
                .get(&p.bucket_id)
                .copied()
                .ok_or_else(|| AppError::NotFound("Bucket not found in this project".into()))
        })
        .collect()
}
async fn active_token_named(
    tx: &DatabaseTransaction,
    project_id: Uuid,
    name: &str,
) -> Result<bool, AppError> {
    Ok(storage_access_token::Entity::find()
        .filter(storage_access_token::Column::ProjectId.eq(project_id))
        .filter(storage_access_token::Column::Name.eq(name))
        .join(
            JoinType::InnerJoin,
            storage_access_token::Relation::Credential.def(),
        )
        .filter(credential::Column::RevokedAt.is_null())
        .one(tx)
        .await?
        .is_some())
}
async fn insert_grants(
    tx: &DatabaseTransaction,
    credential_id: Uuid,
    organization_id: Uuid,
    bucket_ids: &[Uuid],
    permissions: &[BucketPermission],
) -> Result<(), AppError> {
    let grants = bucket_ids
        .iter()
        .zip(permissions)
        .map(|(id, p)| bucket_grant::ActiveModel {
            id: Set(Uuid::new_v4()),
            credential_id: Set(credential_id),
            bucket_id: Set(*id),
            organization_id: Set(Some(organization_id)),
            prefix: Set(String::new()),
            can_read: Set(p.can_read),
            can_write: Set(p.can_write),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    if !grants.is_empty() {
        bucket_grant::Entity::insert_many(grants).exec(tx).await?;
    }
    Ok(())
}
async fn token_by_id(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
    credential_id: Uuid,
) -> Result<AccessToken, AppError> {
    let (t, c) = storage_access_token::Entity::find()
        .filter(storage_access_token::Column::CredentialId.eq(credential_id))
        .filter(storage_access_token::Column::OrganizationId.eq(organization_id))
        .filter(storage_access_token::Column::ProjectId.eq(project_id))
        .find_also_related(credential::Entity)
        .filter(credential::Column::RevokedAt.is_null())
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("S3 access token not found".into()))?;
    let c = c.ok_or_else(|| AppError::NotFound("S3 access token not found".into()))?;
    Ok(AccessToken {
        id: t.credential_id,
        name: t.name,
        access_key_id: c.access_key_id,
        prefix: c.prefix,
        created_at: c.created_at.to_string(),
    })
}
