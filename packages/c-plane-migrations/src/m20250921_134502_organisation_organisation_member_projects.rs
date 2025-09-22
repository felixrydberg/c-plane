use sea_orm_migration::{prelude::{extension::postgres::Type, *}, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(OrganisationRole::Enum)
                    .values([
                        OrganisationRole::Owner,
                        OrganisationRole::Admin,
                        OrganisationRole::Member,
                        OrganisationRole::Viewer,
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Organisation::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Organisation::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(ColumnDef::new(Organisation::Name).string().not_null())
                    .col(ColumnDef::new(Organisation::Description).text())
                    .col(ColumnDef::new(Organisation::AvatarUrl).text())
                    .col(
                        ColumnDef::new(Organisation::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Organisation::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT NOW()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(Organisation::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT NOW()".to_owned()),
                    )
                    .col(ColumnDef::new(Organisation::CreatedBy).uuid().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(OrganisationMember::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OrganisationMember::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(ColumnDef::new(OrganisationMember::OrganisationId).uuid().not_null())
                    .col(ColumnDef::new(OrganisationMember::IdentityId).uuid().not_null())
                    .col(
                        ColumnDef::new(OrganisationMember::Role)
                            .enumeration(OrganisationRole::Enum, [
                                OrganisationRole::Owner,
                                OrganisationRole::Admin,
                                OrganisationRole::Member,
                                OrganisationRole::Viewer,
                            ])
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganisationMember::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(OrganisationMember::JoinedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT NOW()".to_owned()),
                    )
                    .col(ColumnDef::new(OrganisationMember::InvitedBy).uuid().not_null())
                    .col(
                        ColumnDef::new(OrganisationMember::InvitedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT NOW()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(OrganisationMember::InvitationAcceptedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT NOW()".to_owned()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_organisation_members_organisation")
                            .from(OrganisationMember::Table, OrganisationMember::OrganisationId)
                            .to(Organisation::Table, Organisation::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Project::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Project::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(ColumnDef::new(Project::Name).string().not_null())
                    .col(ColumnDef::new(Project::Description).text())
                    .col(ColumnDef::new(Project::OrganisationId).uuid().not_null())
                    .col(ColumnDef::new(Project::OwnerId).uuid().not_null())
                    .col(
                        ColumnDef::new(Project::IsArchived)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Project::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT NOW()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(Project::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT NOW()".to_owned()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_projects_organisation")
                            .from(Project::Table, Project::OrganisationId)
                            .to(Organisation::Table, Organisation::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE OR REPLACE FUNCTION update_updated_at_column()
                RETURNS TRIGGER AS $$
                BEGIN
                    NEW.updated_at = NOW();
                    RETURN NEW;
                END;
                $$ language 'plpgsql';
                "#,
            )
            .await?;

        for table in ["organisation", "project"] {
            let trigger_sql = format!(
                r#"
                CREATE TRIGGER update_{}_updated_at 
                    BEFORE UPDATE ON {} 
                    FOR EACH ROW 
                    EXECUTE FUNCTION update_updated_at_column();
                "#,
                table, table
            );
            manager.get_connection().execute_unprepared(&trigger_sql).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Project::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(OrganisationMember::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Organisation::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(OrganisationRole::Enum).to_owned())
            .await?;

        manager
            .get_connection()
            .execute_unprepared("DROP FUNCTION IF EXISTS update_updated_at_column() CASCADE")
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Organisation {
    Table,
    Id,
    Name,
    Description,
    AvatarUrl,
    IsActive,
    CreatedAt,
    UpdatedAt,
    CreatedBy,
}

#[derive(DeriveIden)]
enum OrganisationMember {
    Table,
    Id,
    OrganisationId,
    IdentityId,
    Role,
    IsActive,
    JoinedAt,
    InvitedBy,
    InvitedAt,
    InvitationAcceptedAt,
}

#[derive(DeriveIden)]
enum Project {
    Table,
    Id,
    Name,
    Description,
    OrganisationId,
    OwnerId,
    IsArchived,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum OrganisationRole {
    Enum,
    Owner,
    Admin,
    Member,
    Viewer,
}
