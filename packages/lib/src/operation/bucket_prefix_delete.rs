use sea_orm::{DatabaseTransaction, DbErr, EntityTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Context, Operation, Result};
use crate::entities::bucket;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BucketPrefixDelete {
    pub provider_id: Uuid,
    pub bucket_id: Uuid,
    pub prefix: String,
}

impl Operation<BucketPrefixDelete> {
    pub const QUEUE: &'static str = "foundation";
    pub const NAME: &'static str = "bucket_prefix_delete";

    pub async fn new(
        transaction: &DatabaseTransaction,
        organization_id: Uuid,
        dedupe_key: String,
        input: BucketPrefixDelete,
    ) -> std::result::Result<Self, DbErr> {
        Self::insert(
            transaction,
            Some(organization_id),
            Self::QUEUE,
            Self::NAME,
            Some(dedupe_key),
            input,
        )
        .await
    }

    pub async fn new_many<I>(
        transaction: &DatabaseTransaction,
        organization_id: Uuid,
        jobs: I,
    ) -> std::result::Result<(), DbErr>
    where
        I: IntoIterator<Item = (String, BucketPrefixDelete)>,
    {
        Self::insert_many(
            transaction,
            Some(organization_id),
            Self::QUEUE,
            Self::NAME,
            jobs.into_iter()
                .map(|(dedupe_key, input)| (Some(dedupe_key), input)),
        )
        .await
    }

    pub async fn run(&self, context: &Context<'_>) -> Result<()> {
        delete_prefix(context, &self.input).await
    }
}

pub async fn delete_prefix(context: &Context<'_>, job: &BucketPrefixDelete) -> Result<()> {
    if bucket::Entity::find_by_id(job.bucket_id)
        .one(context.database)
        .await?
        .is_none()
    {
        return Ok(());
    }

    let client = super::foundation_bucket_delete::provider_client(context, job.provider_id).await?;
    crate::buckets::delete_prefix(
        &client,
        &crate::buckets::physical_bucket_name(job.bucket_id),
        &job.prefix,
        None,
        usize::MAX,
    )
    .await
    .map(|_| ())
}
