export interface paths {
    "/api/organization/{organization_id}/containers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_containers"];
        put?: never;
        post: operations["create_container"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/containers/{container_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_container"];
        put?: never;
        post?: never;
        delete: operations["delete_container"];
        options?: never;
        head?: never;
        patch: operations["update_container"];
        trace?: never;
    };
    "/api/organization/{organization_id}/containers/{container_id}/deploy": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["redeploy_container"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/databases/postgres": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_databases"];
        put?: never;
        post: operations["create_database"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/databases/postgres/{database_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_database"];
        put?: never;
        post?: never;
        delete: operations["delete_database"];
        options?: never;
        head?: never;
        patch: operations["update_database"];
        trace?: never;
    };
    "/api/organization/{organization_id}/databases/postgres/{database_id}/branches": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_database_branches"];
        put?: never;
        post: operations["create_database_branch"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["delete_database_branch"];
        options?: never;
        head?: never;
        patch: operations["update_database_branch"];
        trace?: never;
    };
    "/api/organization/{organization_id}/environments": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_organization_environments"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/events": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_events"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/projects": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_projects"];
        put?: never;
        post: operations["create_project"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/projects/{project_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_project"];
        put?: never;
        post?: never;
        delete: operations["delete_project"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/projects/{project_id}/environments": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_environments"];
        put?: never;
        post: operations["create_environment"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["delete_environment"];
        options?: never;
        head?: never;
        patch: operations["update_environment"];
        trace?: never;
    };
    "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["storage_list_access_tokens"];
        put?: never;
        post: operations["storage_create_access_token"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["storage_get_access_token"];
        put?: never;
        post?: never;
        delete: operations["storage_revoke_access_token"];
        options?: never;
        head?: never;
        patch: operations["storage_update_access_token"];
        trace?: never;
    };
    "/api/organization/{organization_id}/projects/{project_id}/timelines": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_project_timelines"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/projects/{project_id}/timelines/{timeline_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_timeline"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/regions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_regions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/registry/access-tokens": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["registry_list_access_tokens"];
        put?: never;
        post: operations["registry_create_access_token"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/registry/access-tokens/{token_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["registry_get_access_token"];
        put?: never;
        post?: never;
        delete: operations["registry_revoke_access_token"];
        options?: never;
        head?: never;
        patch: operations["registry_update_access_token"];
        trace?: never;
    };
    "/api/organization/{organization_id}/registry/external-registries": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_external_registries"];
        put?: never;
        post: operations["create_external_registry"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/registry/external-registries/{registry_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["delete_external_registry"];
        options?: never;
        head?: never;
        patch: operations["rename_external_registry"];
        trace?: never;
    };
    "/api/organization/{organization_id}/registry/external-registries/{registry_id}/rotate-token": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["rotate_external_registry_token"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/registry/repositories": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_repositories"];
        put?: never;
        post: operations["create_repository"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/registry/repositories/{repository_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["delete_repository"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/storage/buckets": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_buckets"];
        put?: never;
        post: operations["create_bucket"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/storage/buckets/{bucket_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["delete_bucket"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["storage_list_bucket_objects"];
        put?: never;
        post?: never;
        delete: operations["storage_delete_bucket_objects"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects/download": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["storage_download_bucket_object"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/registry/maintenance": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["maintenance_status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/registry/token": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["issue_token"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/health": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["health_check"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/internal/s3-access-tokens/resolve/{access_key}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["resolve_access_token"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/internal/s3-providers/{provider_id}/credentials": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["provider_credentials"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        AccessTokenDetailsResponse: components["schemas"]["AccessTokenResponse"] & {
            bucket_permissions: components["schemas"]["BucketPermissionRequest"][];
        };
        AccessTokenResponse: {
            access_key_id: string;
            created_at: string;
            /** Format: uuid */
            id: string;
            name: string;
        };
        BucketObjectResponse: {
            etag?: string | null;
            key: string;
            last_modified?: string | null;
            /** Format: int64 */
            size: number;
        };
        BucketObjectsResponse: {
            folders: string[];
            next_continuation_token?: string | null;
            objects: components["schemas"]["BucketObjectResponse"][];
        };
        BucketPermissionRequest: {
            /** Format: uuid */
            bucket_id: string;
            can_read: boolean;
            can_write: boolean;
        };
        BucketResponse: {
            /** Format: uuid */
            id: string;
            name: string;
            /** Format: uuid */
            project_id: string;
            /** Format: uuid */
            region: string;
        };
        ContainerResponse: {
            created_at: string;
            current_version?: null | components["schemas"]["ContainerVersionResponse"];
            /** Format: uuid */
            id: string;
            name: string;
            /** Format: uuid */
            organization_id: string;
            /** Format: uuid */
            project_id?: string | null;
            /** Format: uuid */
            region_id: string;
            updated_at: string;
        };
        ContainerVersionResponse: {
            created_at: string;
            env?: unknown;
            /** Format: uuid */
            external_registry_id?: string | null;
            health_check?: unknown;
            /** Format: uuid */
            id: string;
            image: string;
            /** Format: int32 */
            port?: number | null;
            public: boolean;
            /** Format: int32 */
            replica_count: number;
            resolved_image: string;
            resources?: unknown;
            /** Format: int32 */
            version: number;
        };
        CreateAccessTokenRequest: {
            bucket_permissions: components["schemas"]["BucketPermissionRequest"][];
            name: string;
        };
        CreateBucketRequest: {
            name: string;
            /** Format: uuid */
            project_id: string;
            /** Format: uuid */
            region: string;
        };
        CreateContainerRequest: {
            auto_deploy?: boolean;
            env?: unknown;
            /** Format: uuid */
            environment_id: string;
            /** Format: uuid */
            external_registry_id?: string | null;
            health_check?: unknown;
            image: string;
            name: string;
            /** Format: int32 */
            port?: number | null;
            /** Format: uuid */
            project_id: string;
            public?: boolean;
            /** Format: uuid */
            region_id: string;
            /** Format: int32 */
            replica_count?: number;
            resources?: unknown;
        };
        CreateDatabaseBranchRequest: {
            autoscaling_enabled?: boolean | null;
            autoscaling_max_cpu?: string | null;
            autoscaling_min_cpu?: string | null;
            /** Format: int32 */
            backup_retention_days?: number | null;
            /** Format: uuid */
            branch_id: string;
            cpu?: string | null;
            high_availability?: boolean | null;
            ram?: string | null;
            /** Format: int32 */
            read_replicas?: number | null;
        };
        CreateDatabaseRequest: {
            autoscaling_enabled?: boolean;
            autoscaling_max_cpu?: string | null;
            autoscaling_min_cpu?: string | null;
            /** Format: int32 */
            backup_retention_days?: number | null;
            cpu?: string | null;
            high_availability?: boolean;
            name: string;
            /** Format: uuid */
            project_id: string;
            ram?: string | null;
            /** Format: int32 */
            read_replicas?: number | null;
        };
        CreateEnvironmentRequest: {
            is_preview?: boolean;
            name: string;
            /** Format: uuid */
            parent_timeline_id?: string | null;
        };
        CreateExternalRegistryRequest: {
            host?: string | null;
            name: string;
            provider: components["schemas"]["ExternalRegistryProvider"];
            token: string;
            username: string;
        };
        CreateProjectRequest: {
            name: string;
        };
        CreateRegistryAccessTokenRequest: {
            name: string;
            repository_permissions: components["schemas"]["RepositoryPermissionRequest"][];
        };
        CreateRegistryRepositoryRequest: {
            name: string;
        };
        CreatedAccessTokenResponse: components["schemas"]["AccessTokenResponse"] & {
            endpoint_url: string;
            secret_access_key: string;
        };
        CreatedRegistryAccessTokenResponse: components["schemas"]["RegistryAccessTokenResponse"] & {
            token: string;
        };
        DatabaseBranchResponse: {
            autoscaling_enabled: boolean;
            autoscaling_max_cpu?: string | null;
            autoscaling_min_cpu?: string | null;
            /** Format: int32 */
            backup_retention_days?: number | null;
            /** Format: uuid */
            branch_id: string;
            cpu?: string | null;
            /** Format: uuid */
            database_id: string;
            high_availability: boolean;
            /** Format: uuid */
            id: string;
            /** Format: uuid */
            organization_id: string;
            ram?: string | null;
            /** Format: int32 */
            read_replicas?: number | null;
        };
        DatabaseResponse: {
            /** Format: uuid */
            default_branch_id?: string | null;
            /** Format: uuid */
            id: string;
            name: string;
            /** Format: uuid */
            project_id: string;
        };
        DeleteObjectsResponse: {
            deleted: number;
            next_continuation_token?: string | null;
        };
        EnvironmentResponse: {
            deployed_timeline: string;
            draft_timeline: string;
            /** Format: uuid */
            id: string;
            is_default: boolean;
            is_preview: boolean;
            name: string;
        };
        EnvironmentWithProjectResponse: {
            deployed_timeline: string;
            draft_timeline: string;
            /** Format: uuid */
            id: string;
            is_default: boolean;
            is_preview: boolean;
            name: string;
            /** Format: uuid */
            project_id: string;
            project_name: string;
        };
        ErrorResponse: {
            details?: unknown;
            error: string;
            message: string;
        };
        EventResponse: {
            action: string;
            /** Format: uuid */
            actor_id?: string | null;
            actor_name?: string | null;
            created_at: string;
            /** Format: uuid */
            id: string;
            summary: string;
        };
        /** @enum {string} */
        ExternalRegistryProvider: "docker_hub" | "github" | "gitlab" | "google_artifact_registry" | "aws_ecr";
        ExternalRegistryResponse: {
            created_at: string;
            host: string;
            /** Format: uuid */
            id: string;
            name: string;
            /** Format: uuid */
            organization_id: string;
            updated_at: string;
            username: string;
        };
        HealthResponse: {
            status: string;
        };
        PaginatedResponse_ProjectResponse: {
            data: {
                created_at: string;
                /** Format: uuid */
                default_environment_id?: string | null;
                /** Format: uuid */
                id: string;
                main_environment?: null | components["schemas"]["EnvironmentResponse"];
                name: string;
                /** Format: uuid */
                organization_id: string;
                updated_at: string;
            }[];
            pagination: components["schemas"]["PaginationMeta"];
        };
        PaginationMeta: {
            has_next: boolean;
            has_prev: boolean;
            /** Format: int64 */
            page: number;
            /** Format: int64 */
            per_page: number;
            /** Format: int64 */
            total: number;
            /** Format: int64 */
            total_pages: number;
        };
        ProjectResponse: {
            created_at: string;
            /** Format: uuid */
            default_environment_id?: string | null;
            /** Format: uuid */
            id: string;
            main_environment?: null | components["schemas"]["EnvironmentResponse"];
            name: string;
            /** Format: uuid */
            organization_id: string;
            updated_at: string;
        };
        RegionResponse: {
            display_name: string;
            /** Format: uuid */
            id: string;
        };
        RegistryAccessTokenDetailsResponse: components["schemas"]["RegistryAccessTokenResponse"] & {
            repository_permissions: components["schemas"]["RepositoryPermissionRequest"][];
        };
        RegistryAccessTokenResponse: {
            created_at: string;
            /** Format: uuid */
            id: string;
            name: string;
        };
        RegistryMaintenanceResponse: {
            phase?: string | null;
            read_only: boolean;
            started_at?: string | null;
        };
        RegistryRepositoryResponse: {
            created_at: string;
            /** Format: uuid */
            id: string;
            name: string;
        };
        RegistryTokenResponse: {
            access_token: string;
            /** Format: int64 */
            expires_in: number;
            issued_at: string;
            token: string;
        };
        RenameExternalRegistryRequest: {
            name: string;
        };
        RepositoryPermissionRequest: {
            can_pull: boolean;
            can_push: boolean;
            /** Format: uuid */
            repository_id: string;
        };
        ResolvedContainerPin: {
            /** Format: uuid */
            container_id: string;
            container_name: string;
            /** Format: uuid */
            external_registry_id?: string | null;
            image: string;
            /** Format: int32 */
            version: number;
            /** Format: uuid */
            version_id: string;
        };
        ResolvedS3AccessToken: {
            bucket_permissions: components["schemas"]["ResolvedS3BucketPermission"][];
            /** Format: uuid */
            credential_id: string;
            /** Format: uuid */
            organization_id?: string | null;
            /** Format: uuid */
            project_id?: string | null;
        };
        ResolvedS3BucketPermission: {
            /** Format: uuid */
            bucket_id: string;
            bucket_name: string;
            can_read: boolean;
            can_write: boolean;
            physical_bucket_name: string;
            /** Format: uuid */
            provider_id: string;
            region: string;
        };
        ResolvedTimelineResponse: {
            containers: components["schemas"]["ResolvedContainerPin"][];
            created_at: string;
            /** Format: uuid */
            environment_id?: string | null;
            /** Format: uuid */
            id: string;
            name?: string | null;
            /** Format: uuid */
            parent_timeline_id?: string | null;
            /** Format: int32 */
            timeline: number;
        };
        RotateExternalRegistryTokenRequest: {
            token: string;
        };
        S3ProviderCredentials: {
            access_key_id: string;
            endpoint_url: string;
            provider_region?: string | null;
            name: string;
        };
        TimelineResponse: {
            created_at: string;
            /** Format: uuid */
            environment_id?: string | null;
            /** Format: uuid */
            id: string;
            name?: string | null;
            /** Format: uuid */
            parent_timeline_id?: string | null;
            pins: unknown;
            /** Format: int32 */
            timeline: number;
        };
        UpdateAccessTokenRequest: {
            bucket_permissions: components["schemas"]["BucketPermissionRequest"][];
        };
        UpdateContainerRequest: {
            auto_deploy?: boolean;
            env?: unknown;
            /** Format: uuid */
            external_registry_id?: string | null;
            health_check?: unknown;
            image?: string | null;
            name?: string | null;
            /** Format: int32 */
            port?: number | null;
            public?: boolean | null;
            /** Format: int32 */
            replica_count?: number | null;
            resources?: unknown;
        };
        UpdateDatabaseBranchRequest: {
            autoscaling_enabled?: boolean | null;
            autoscaling_max_cpu?: string | null;
            autoscaling_min_cpu?: string | null;
            /** Format: int32 */
            backup_retention_days?: number | null;
            cpu?: string | null;
            high_availability?: boolean | null;
            ram?: string | null;
            /** Format: int32 */
            read_replicas?: number | null;
        };
        UpdateDatabaseRequest: {
            name?: string | null;
        };
        UpdateEnvironmentRequest: {
            /** Format: uuid */
            deployed_timeline_id?: string | null;
            /** Format: uuid */
            draft_timeline_id?: string | null;
            name?: string | null;
        };
        UpdateRegistryAccessTokenRequest: {
            repository_permissions: components["schemas"]["RepositoryPermissionRequest"][];
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    list_containers: {
        parameters: {
            query?: {
                /** @description Filter by project */
                project_id?: string;
                /** @description Filter by environment */
                environment_id?: string;
                /** @description Revision whose pinned containers to return */
                timeline_id?: string;
            };
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of containers */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ContainerResponse"][];
                };
            };
        };
    };
    create_container: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateContainerRequest"];
            };
        };
        responses: {
            /** @description Container created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ContainerResponse"];
                };
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_container: {
        parameters: {
            query?: {
                /** @description Environment that owns the revision history */
                environment_id?: string;
                /** @description Revision whose pinned container version to return */
                timeline_id?: string;
            };
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Container ID */
                container_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Container details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ContainerResponse"];
                };
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    delete_container: {
        parameters: {
            query: {
                /** @description Environment ID for the revision */
                environment_id: string;
                /** @description Draft revision to remove the container from */
                timeline_id: string;
                /** @description Deploy the removal immediately */
                deploy?: boolean;
            };
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Container ID */
                container_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Container deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    update_container: {
        parameters: {
            query: {
                /** @description Environment ID for the revision */
                environment_id: string;
                /** @description Revision that supplies the container update base */
                timeline_id: string;
            };
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Container ID */
                container_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateContainerRequest"];
            };
        };
        responses: {
            /** @description Container updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ContainerResponse"];
                };
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    redeploy_container: {
        parameters: {
            query: {
                /** @description Environment ID for the revision */
                environment_id: string;
                /** @description Draft revision to deploy */
                timeline_id: string;
            };
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Container ID */
                container_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Container redeployed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ContainerResponse"];
                };
            };
            /** @description Container image is not refreshable */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_databases: {
        parameters: {
            query: {
                /** @description Project ID */
                project_id: string;
            };
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Project databases */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DatabaseResponse"][];
                };
            };
        };
    };
    create_database: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateDatabaseRequest"];
            };
        };
        responses: {
            /** @description Database created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DatabaseResponse"];
                };
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_database: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Database ID */
                database_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Database details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DatabaseResponse"];
                };
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    delete_database: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Database ID */
                database_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Database deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    update_database: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Database ID */
                database_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateDatabaseRequest"];
            };
        };
        responses: {
            /** @description Database updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DatabaseResponse"];
                };
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_database_branches: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Database ID */
                database_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Database branch links */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DatabaseBranchResponse"][];
                };
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_database_branch: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Database ID */
                database_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateDatabaseBranchRequest"];
            };
        };
        responses: {
            /** @description Link already exists */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DatabaseBranchResponse"];
                };
            };
            /** @description Database branch link created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DatabaseBranchResponse"];
                };
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    delete_database_branch: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Database ID */
                database_id: string;
                /** @description Branch ID */
                branch_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Database branch link deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    update_database_branch: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Database ID */
                database_id: string;
                /** @description Branch ID */
                branch_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateDatabaseBranchRequest"];
            };
        };
        responses: {
            /** @description Database branch updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DatabaseBranchResponse"];
                };
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_organization_environments: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Organization environments */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["EnvironmentWithProjectResponse"][];
                };
            };
        };
    };
    list_events: {
        parameters: {
            query?: {
                /** @description Optional project ID filter */
                project_id?: string;
                /** @description Event type prefix filter (e.g. 'container' matches 'container:created') */
                event_type_prefix?: string;
                /** @description Environment filter (matched against payload->>'environment_id') */
                environment_id?: string;
                /** @description Resource ID filter (matched against payload->>'target_id') */
                target_id?: string;
                /** @description Maximum events (default 10, max 50) */
                limit?: number;
            };
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["EventResponse"][];
                };
            };
        };
    };
    list_projects: {
        parameters: {
            query?: {
                /** @description Project name search */
                search?: string;
                /** @description Page number */
                page?: number;
                /** @description Items per page */
                per_page?: number;
            };
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Paginated projects */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse_ProjectResponse"];
                };
            };
        };
    };
    create_project: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateProjectRequest"];
            };
        };
        responses: {
            /** @description Project created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProjectResponse"];
                };
            };
            /** @description Organization not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_project: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Project ID */
                project_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Project details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProjectResponse"];
                };
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    delete_project: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Project ID */
                project_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Project deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_environments: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Project ID */
                project_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Project environments */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["EnvironmentResponse"][];
                };
            };
        };
    };
    create_environment: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Project ID */
                project_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateEnvironmentRequest"];
            };
        };
        responses: {
            /** @description Environment created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["EnvironmentResponse"];
                };
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Environment name already exists */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    delete_environment: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Project ID */
                project_id: string;
                /** @description Environment ID */
                environment_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Environment deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Cannot delete default environment */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    update_environment: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Project ID */
                project_id: string;
                /** @description Environment ID */
                environment_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateEnvironmentRequest"];
            };
        };
        responses: {
            /** @description Environment updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["EnvironmentResponse"];
                };
            };
            /** @description Name or revision is required */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Environment name already exists */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    storage_list_access_tokens: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Project ID */
                project_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of active access tokens */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AccessTokenResponse"][];
                };
            };
            /** @description Project not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    storage_create_access_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Project ID */
                project_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateAccessTokenRequest"];
            };
        };
        responses: {
            /** @description Access token created; save the secret access key now */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CreatedAccessTokenResponse"];
                };
            };
            /** @description Project or bucket not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Invalid permissions or duplicate token name */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    storage_get_access_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Project ID */
                project_id: string;
                /** @description Access token ID */
                token_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Access token details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AccessTokenDetailsResponse"];
                };
            };
            /** @description Project or access token not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    storage_revoke_access_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Project ID */
                project_id: string;
                /** @description Access token ID */
                token_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Access token revoked */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Project or access token not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    storage_update_access_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Project ID */
                project_id: string;
                /** @description Access token ID */
                token_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateAccessTokenRequest"];
            };
        };
        responses: {
            /** @description Access token permissions updated */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Project, access token, or bucket not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Invalid permissions */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_project_timelines: {
        parameters: {
            query?: {
                /** @description Environment ID */
                environment_id?: string;
            };
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Project ID */
                project_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Project timelines */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TimelineResponse"][];
                };
            };
        };
    };
    get_timeline: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Project ID */
                project_id: string;
                /** @description Timeline ID */
                timeline_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Resolved timeline */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ResolvedTimelineResponse"];
                };
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_regions: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Available regions */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RegionResponse"][];
                };
            };
        };
    };
    registry_list_access_tokens: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of active registry access tokens */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RegistryAccessTokenResponse"][];
                };
            };
            /** @description Organization access required */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    registry_create_access_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateRegistryAccessTokenRequest"];
            };
        };
        responses: {
            /** @description Registry access token created; save it now */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CreatedRegistryAccessTokenResponse"];
                };
            };
            /** @description Organization access required */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Repository not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Invalid permissions or duplicate token name */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    registry_get_access_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Registry access token ID */
                token_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Registry access token details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RegistryAccessTokenDetailsResponse"];
                };
            };
            /** @description Registry access token not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    registry_revoke_access_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Registry access token ID */
                token_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Registry access token revoked */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Organization access required */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Registry access token not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    registry_update_access_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Registry access token ID */
                token_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateRegistryAccessTokenRequest"];
            };
        };
        responses: {
            /** @description Registry access token permissions updated */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Organization access required */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Registry access token or repository not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Invalid permissions */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_external_registries: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ExternalRegistryResponse"][];
                };
            };
        };
    };
    create_external_registry: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateExternalRegistryRequest"];
            };
        };
        responses: {
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ExternalRegistryResponse"];
                };
            };
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorResponse"];
                };
            };
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorResponse"];
                };
            };
        };
    };
    delete_external_registry: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                organization_id: string;
                registry_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorResponse"];
                };
            };
        };
    };
    rename_external_registry: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                organization_id: string;
                registry_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RenameExternalRegistryRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ExternalRegistryResponse"];
                };
            };
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorResponse"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorResponse"];
                };
            };
        };
    };
    rotate_external_registry_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                organization_id: string;
                registry_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RotateExternalRegistryTokenRequest"];
            };
        };
        responses: {
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorResponse"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorResponse"];
                };
            };
        };
    };
    list_repositories: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Organization registry repositories */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RegistryRepositoryResponse"][];
                };
            };
            /** @description Organization access required */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_repository: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateRegistryRepositoryRequest"];
            };
        };
        responses: {
            /** @description Registry repository created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RegistryRepositoryResponse"];
                };
            };
            /** @description Organization access required */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Invalid or duplicate repository name */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    delete_repository: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Registry repository ID */
                repository_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Registry repository deleted */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Organization access required */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Registry repository not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Registry cleanup conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Registry cleanup failed */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Registry is read-only for maintenance */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorResponse"];
                };
            };
        };
    };
    list_buckets: {
        parameters: {
            query: {
                /** @description Project ID */
                project_id: string;
            };
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of buckets */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BucketResponse"][];
                };
            };
            /** @description Project not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_bucket: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateBucketRequest"];
            };
        };
        responses: {
            /** @description Bucket created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BucketResponse"];
                };
            };
            /** @description Project or active region not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Bucket name is unavailable */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    delete_bucket: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Bucket ID */
                bucket_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Bucket deleted */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Bucket or region not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Provider bucket could not be deleted */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    storage_list_bucket_objects: {
        parameters: {
            query?: {
                /** @description Folder prefix */
                prefix?: string;
                /** @description S3 continuation token */
                continuation_token?: string;
            };
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Bucket ID */
                bucket_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Objects and folders in the bucket */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BucketObjectsResponse"];
                };
            };
            /** @description Bucket not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Storage gateway unavailable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    storage_delete_bucket_objects: {
        parameters: {
            query?: {
                /** @description Exact object key */
                key?: string;
                /** @description Folder prefix */
                prefix?: string;
                /** @description Continuation token for a partial folder deletion */
                continuation_token?: string;
            };
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Bucket ID */
                bucket_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Folder deletion partially completed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DeleteObjectsResponse"];
                };
            };
            /** @description Object or folder deleted */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Exactly one non-empty key or prefix is required */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Bucket not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Storage gateway unavailable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    storage_download_bucket_object: {
        parameters: {
            query: {
                /** @description Exact object key */
                key: string;
            };
            header?: never;
            path: {
                /** @description Organization ID */
                organization_id: string;
                /** @description Bucket ID */
                bucket_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Object download */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/octet-stream": Blob;
                };
            };
            /** @description Object key is required */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Bucket or object not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Storage gateway unavailable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    maintenance_status: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Registry maintenance state */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RegistryMaintenanceResponse"];
                };
            };
        };
    };
    issue_token: {
        parameters: {
            query: {
                /** @description Distribution service name */
                service: string;
                /** @description Requested repository scopes */
                scope: string[];
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Short-lived Distribution access token */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RegistryTokenResponse"];
                };
            };
            /** @description Invalid registry credentials */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Registry is read-only for maintenance */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorResponse"];
                };
            };
        };
    };
    health_check: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description API health status */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HealthResponse"];
                };
            };
        };
    };
    resolve_access_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                access_key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ResolvedS3AccessToken"];
                };
            };
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorResponse"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorResponse"];
                };
            };
        };
    };
    provider_credentials: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                provider_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["S3ProviderCredentials"];
                };
            };
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorResponse"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorResponse"];
                };
            };
        };
    };
}
