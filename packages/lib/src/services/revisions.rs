use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

use crate::entities::{
    external_registry, project, project_environment, project_revision_manifest, project_timeline,
};
use crate::error::AppError;
use crate::manifest::RevisionManifest;

pub async fn insert_revision(
    tx: &DatabaseTransaction,
    project_id: Uuid,
    organization_id: Uuid,
    environment_id: Option<Uuid>,
    parent_timeline_id: Option<Uuid>,
    manifest: &RevisionManifest,
    name: Option<String>,
) -> Result<project_timeline::Model, AppError> {
    lock_project(tx, project_id).await?;
    let registry_ids = manifest.external_registry_ids();
    lock_referenced_registries(tx, organization_id, &registry_ids).await?;

    let manifest_id =
        insert_manifest(tx, project_id, organization_id, manifest, registry_ids).await?;
    let timeline = next_revision_number(tx, project_id).await?;

    project_timeline::ActiveModel {
        id: Set(Uuid::new_v4()),
        project_id: Set(project_id),
        environment_id: Set(environment_id),
        organization_id: Set(organization_id),
        timeline: Set(timeline),
        name: Set(name),
        parent_timeline_id: Set(parent_timeline_id),
        manifest_id: Set(manifest_id),
        created_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await
    .map_err(AppError::from)
}

pub async fn create_revision(
    tx: &DatabaseTransaction,
    environment: &project_environment::Model,
    manifest: &RevisionManifest,
    name: Option<String>,
    deploy: bool,
) -> Result<project_timeline::Model, AppError> {
    lock_project(tx, environment.project_id).await?;
    let current = project_environment::Entity::find_by_id(environment.id)
        .filter(project_environment::Column::ProjectId.eq(environment.project_id))
        .lock_exclusive()
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Environment not found".into()))?;
    if current.draft_timeline != environment.draft_timeline {
        return Err(AppError::Conflict(
            "Environment draft changed; reload before editing".into(),
        ));
    }
    let inserted = insert_revision(
        tx,
        environment.project_id,
        environment.organization_id,
        Some(environment.id),
        Some(environment.draft_timeline),
        manifest,
        name,
    )
    .await?;

    let mut environment_active: project_environment::ActiveModel = environment.clone().into();
    environment_active.draft_timeline = Set(inserted.id);
    if deploy {
        environment_active.deployed_timeline = Set(inserted.id);
    }
    environment_active.updated_at = Set(Utc::now().fixed_offset());
    environment_active.update(tx).await?;

    Ok(inserted)
}

pub async fn load_manifest(
    conn: &impl ConnectionTrait,
    manifest_id: Uuid,
) -> Result<RevisionManifest, AppError> {
    let row = project_revision_manifest::Entity::find_by_id(manifest_id)
        .one(conn)
        .await?
        .ok_or_else(|| AppError::NotFound("Revision manifest not found".into()))?;
    RevisionManifest::from_json_value(&row.configuration, row.schema_version)
}

async fn insert_manifest(
    tx: &DatabaseTransaction,
    project_id: Uuid,
    organization_id: Uuid,
    manifest: &RevisionManifest,
    registry_ids: Vec<Uuid>,
) -> Result<Uuid, AppError> {
    let id = Uuid::new_v4();
    project_revision_manifest::ActiveModel {
        id: Set(id),
        project_id: Set(project_id),
        organization_id: Set(organization_id),
        schema_version: Set(manifest.schema_version),
        configuration: Set(manifest.to_json_value()?),
        external_registry_ids: Set(registry_ids),
        created_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;
    Ok(id)
}

pub async fn lock_project(tx: &DatabaseTransaction, project_id: Uuid) -> Result<(), AppError> {
    project::Entity::find_by_id(project_id)
        .lock_exclusive()
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".into()))?;
    Ok(())
}

async fn next_revision_number(tx: &DatabaseTransaction, project_id: Uuid) -> Result<i32, AppError> {
    let max = project_timeline::Entity::find()
        .filter(project_timeline::Column::ProjectId.eq(project_id))
        .order_by_desc(project_timeline::Column::Timeline)
        .one(tx)
        .await?
        .map(|row| row.timeline)
        .unwrap_or(0);

    Ok(max + 1)
}

async fn lock_referenced_registries(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    ids: &[Uuid],
) -> Result<(), AppError> {
    for id in ids {
        external_registry::Entity::find_by_id(*id)
            .filter(external_registry::Column::OrganizationId.eq(organization_id))
            .lock_exclusive()
            .one(tx)
            .await?
            .ok_or_else(|| AppError::NotFound("External registry not found".into()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Database, TransactionTrait};

    #[tokio::test]
    #[ignore = "Requires a disposable migrated database in REVISION_TEST_DATABASE_URL"]
    async fn shared_revision_edit_preserves_other_environment_and_rejects_stale_draft() {
        let db = Database::connect(std::env::var("REVISION_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        let tx = db.begin().await.unwrap();
        tx.execute_unprepared(r#"
            INSERT INTO organization(id,name,email,slug) VALUES(md5('review-org')::uuid,'Review','review@example.test','review');
            INSERT INTO project(id,organization_id,name) VALUES(md5('review-project')::uuid,md5('review-org')::uuid,'Project');
            INSERT INTO project_revision_manifest(id,project_id,organization_id,schema_version,configuration)
            VALUES(md5('review-manifest')::uuid,md5('review-project')::uuid,md5('review-org')::uuid,1,'{"schema_version":1,"containers":{}}');
            INSERT INTO project_timeline(id,project_id,organization_id,timeline,manifest_id)
            VALUES(md5('review-revision')::uuid,md5('review-project')::uuid,md5('review-org')::uuid,1,md5('review-manifest')::uuid);
            INSERT INTO project_environment(id,project_id,organization_id,name,draft_timeline,deployed_timeline)
            SELECT md5(name)::uuid,md5('review-project')::uuid,md5('review-org')::uuid,name,md5('review-revision')::uuid,md5('review-revision')::uuid
            FROM (VALUES ('first'),('second')) names(name);
        "#).await.unwrap();
        let first = project_environment::Entity::find()
            .filter(project_environment::Column::Name.eq("first"))
            .one(&tx)
            .await
            .unwrap()
            .unwrap();
        let original = project_timeline::Entity::find_by_id(first.draft_timeline)
            .one(&tx)
            .await
            .unwrap()
            .unwrap();
        let saved = load_manifest(&tx, original.manifest_id).await.unwrap();
        let mut edited = saved.clone();
        edited.containers.insert(
            Uuid::new_v4(),
            serde_json::from_value(serde_json::json!({
                "name": "web", "region_id": Uuid::nil(), "image": "app:latest",
                "resolved_image": "app@sha256:fixed", "replica_count": 1, "public": false
            }))
            .unwrap(),
        );
        let revision = create_revision(&tx, &first, &edited, None, false)
            .await
            .unwrap();
        assert_eq!(revision.timeline, 2);
        let updated = project_environment::Entity::find_by_id(first.id)
            .one(&tx)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.draft_timeline, revision.id);
        assert_eq!(updated.deployed_timeline, original.id);
        assert_eq!(
            load_manifest(&tx, revision.manifest_id).await.unwrap(),
            edited
        );
        assert_eq!(
            load_manifest(&tx, original.manifest_id).await.unwrap(),
            saved
        );
        let second = project_environment::Entity::find()
            .filter(project_environment::Column::Name.eq("second"))
            .one(&tx)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(second.draft_timeline, original.id);
        assert_eq!(second.deployed_timeline, original.id);
        assert!(matches!(
            create_revision(&tx, &first, &edited, None, false).await,
            Err(AppError::Conflict(_))
        ));
        tx.rollback().await.unwrap();
    }
}
