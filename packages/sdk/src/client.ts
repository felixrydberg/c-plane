import type { ClientForPath } from 'openapi-fetch'
import type { paths } from './generated.ts'
import { createRawClient, type SdkOptions } from './runtime.ts'

type Operation<Path extends keyof paths, Method extends keyof paths[Path] & string> =
  ClientForPath<paths[Path], 'application/json'>[
    Uppercase<Method> & keyof ClientForPath<paths[Path], 'application/json'>
  ]

export type Sdk = {
  containers: {
    create_container: Operation<"/api/organization/{organization_id}/containers", 'post'>
    delete_container: Operation<"/api/organization/{organization_id}/containers/{container_id}", 'delete'>
    get_container: Operation<"/api/organization/{organization_id}/containers/{container_id}", 'get'>
    list_containers: Operation<"/api/organization/{organization_id}/containers", 'get'>
    redeploy_container: Operation<"/api/organization/{organization_id}/containers/{container_id}/deploy", 'post'>
    update_container: Operation<"/api/organization/{organization_id}/containers/{container_id}", 'patch'>
  }
  databases: {
    postgres: {
      create_database: Operation<"/api/organization/{organization_id}/databases/postgres", 'post'>
      create_database_branch: Operation<"/api/organization/{organization_id}/databases/postgres/{database_id}/branches", 'post'>
      delete_database: Operation<"/api/organization/{organization_id}/databases/postgres/{database_id}", 'delete'>
      delete_database_branch: Operation<"/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}", 'delete'>
      get_database: Operation<"/api/organization/{organization_id}/databases/postgres/{database_id}", 'get'>
      list_database_branches: Operation<"/api/organization/{organization_id}/databases/postgres/{database_id}/branches", 'get'>
      list_databases: Operation<"/api/organization/{organization_id}/databases/postgres", 'get'>
      update_database: Operation<"/api/organization/{organization_id}/databases/postgres/{database_id}", 'patch'>
      update_database_branch: Operation<"/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}", 'patch'>
    }
  }
  environments: {
    create_environment: Operation<"/api/organization/{organization_id}/projects/{project_id}/environments", 'post'>
    delete_environment: Operation<"/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}", 'delete'>
    list_environments: Operation<"/api/organization/{organization_id}/projects/{project_id}/environments", 'get'>
    list_organization_environments: Operation<"/api/organization/{organization_id}/environments", 'get'>
    update_environment: Operation<"/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}", 'patch'>
  }
  events: {
    list_events: Operation<"/api/organization/{organization_id}/events", 'get'>
  }
  health: {
    check: Operation<"/health", 'get'>
  }
  internal: {
    provider_credentials: Operation<"/internal/s3-providers/{provider_id}/credentials", 'get'>
    provision_tenant_key: Operation<"/internal/organizations/{organization_id}/transit-key", 'post'>
    resolve_access_token: Operation<"/internal/s3-access-tokens/resolve/{access_key}", 'get'>
  }
  projects: {
    create_project: Operation<"/api/organization/{organization_id}/projects", 'post'>
    delete_project: Operation<"/api/organization/{organization_id}/projects/{project_id}", 'delete'>
    get_project: Operation<"/api/organization/{organization_id}/projects/{project_id}", 'get'>
    get_timeline: Operation<"/api/organization/{organization_id}/projects/{project_id}/timelines/{timeline_id}", 'get'>
    list_project_timelines: Operation<"/api/organization/{organization_id}/projects/{project_id}/timelines", 'get'>
    list_projects: Operation<"/api/organization/{organization_id}/projects", 'get'>
  }
  regions: {
    list_regions: Operation<"/api/organization/{organization_id}/regions", 'get'>
  }
  registry: {
    create_access_token: Operation<"/api/organization/{organization_id}/registry/access-tokens", 'post'>
    create_external_registry: Operation<"/api/organization/{organization_id}/registry/external-registries", 'post'>
    create_repository: Operation<"/api/organization/{organization_id}/registry/repositories", 'post'>
    delete_external_registry: Operation<"/api/organization/{organization_id}/registry/external-registries/{registry_id}", 'delete'>
    delete_repository: Operation<"/api/organization/{organization_id}/registry/repositories/{repository_id}", 'delete'>
    get_access_token: Operation<"/api/organization/{organization_id}/registry/access-tokens/{token_id}", 'get'>
    issue_token: Operation<"/api/registry/token", 'get'>
    list_access_tokens: Operation<"/api/organization/{organization_id}/registry/access-tokens", 'get'>
    list_external_registries: Operation<"/api/organization/{organization_id}/registry/external-registries", 'get'>
    list_repositories: Operation<"/api/organization/{organization_id}/registry/repositories", 'get'>
    maintenance_status: Operation<"/api/registry/maintenance", 'get'>
    rename_external_registry: Operation<"/api/organization/{organization_id}/registry/external-registries/{registry_id}", 'patch'>
    revoke_access_token: Operation<"/api/organization/{organization_id}/registry/access-tokens/{token_id}", 'delete'>
    rotate_external_registry_token: Operation<"/api/organization/{organization_id}/registry/external-registries/{registry_id}/rotate-token", 'post'>
    update_access_token: Operation<"/api/organization/{organization_id}/registry/access-tokens/{token_id}", 'patch'>
  }
  storage: {
    create_access_token: Operation<"/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens", 'post'>
    create_bucket: Operation<"/api/organization/{organization_id}/storage/buckets", 'post'>
    delete_bucket: Operation<"/api/organization/{organization_id}/storage/buckets/{bucket_id}", 'delete'>
    delete_bucket_objects: Operation<"/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects", 'delete'>
    download_bucket_object: Operation<"/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects/download", 'get'>
    get_access_token: Operation<"/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}", 'get'>
    list_access_tokens: Operation<"/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens", 'get'>
    list_bucket_objects: Operation<"/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects", 'get'>
    list_buckets: Operation<"/api/organization/{organization_id}/storage/buckets", 'get'>
    revoke_access_token: Operation<"/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}", 'delete'>
    update_access_token: Operation<"/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}", 'patch'>
  }
}

export const createSdk = (options: SdkOptions = {}): Sdk => {
  const client = createRawClient(options)
  return {
    containers: {
      create_container: (...args: Parameters<Operation<"/api/organization/{organization_id}/containers", 'post'>>) => client.POST("/api/organization/{organization_id}/containers", ...args),
      delete_container: (...args: Parameters<Operation<"/api/organization/{organization_id}/containers/{container_id}", 'delete'>>) => client.DELETE("/api/organization/{organization_id}/containers/{container_id}", ...args),
      get_container: (...args: Parameters<Operation<"/api/organization/{organization_id}/containers/{container_id}", 'get'>>) => client.GET("/api/organization/{organization_id}/containers/{container_id}", ...args),
      list_containers: (...args: Parameters<Operation<"/api/organization/{organization_id}/containers", 'get'>>) => client.GET("/api/organization/{organization_id}/containers", ...args),
      redeploy_container: (...args: Parameters<Operation<"/api/organization/{organization_id}/containers/{container_id}/deploy", 'post'>>) => client.POST("/api/organization/{organization_id}/containers/{container_id}/deploy", ...args),
      update_container: (...args: Parameters<Operation<"/api/organization/{organization_id}/containers/{container_id}", 'patch'>>) => client.PATCH("/api/organization/{organization_id}/containers/{container_id}", ...args)
    },
    databases: {
      postgres: {
        create_database: (...args: Parameters<Operation<"/api/organization/{organization_id}/databases/postgres", 'post'>>) => client.POST("/api/organization/{organization_id}/databases/postgres", ...args),
        create_database_branch: (...args: Parameters<Operation<"/api/organization/{organization_id}/databases/postgres/{database_id}/branches", 'post'>>) => client.POST("/api/organization/{organization_id}/databases/postgres/{database_id}/branches", ...args),
        delete_database: (...args: Parameters<Operation<"/api/organization/{organization_id}/databases/postgres/{database_id}", 'delete'>>) => client.DELETE("/api/organization/{organization_id}/databases/postgres/{database_id}", ...args),
        delete_database_branch: (...args: Parameters<Operation<"/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}", 'delete'>>) => client.DELETE("/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}", ...args),
        get_database: (...args: Parameters<Operation<"/api/organization/{organization_id}/databases/postgres/{database_id}", 'get'>>) => client.GET("/api/organization/{organization_id}/databases/postgres/{database_id}", ...args),
        list_database_branches: (...args: Parameters<Operation<"/api/organization/{organization_id}/databases/postgres/{database_id}/branches", 'get'>>) => client.GET("/api/organization/{organization_id}/databases/postgres/{database_id}/branches", ...args),
        list_databases: (...args: Parameters<Operation<"/api/organization/{organization_id}/databases/postgres", 'get'>>) => client.GET("/api/organization/{organization_id}/databases/postgres", ...args),
        update_database: (...args: Parameters<Operation<"/api/organization/{organization_id}/databases/postgres/{database_id}", 'patch'>>) => client.PATCH("/api/organization/{organization_id}/databases/postgres/{database_id}", ...args),
        update_database_branch: (...args: Parameters<Operation<"/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}", 'patch'>>) => client.PATCH("/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}", ...args)
      }
    },
    environments: {
      create_environment: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects/{project_id}/environments", 'post'>>) => client.POST("/api/organization/{organization_id}/projects/{project_id}/environments", ...args),
      delete_environment: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}", 'delete'>>) => client.DELETE("/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}", ...args),
      list_environments: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects/{project_id}/environments", 'get'>>) => client.GET("/api/organization/{organization_id}/projects/{project_id}/environments", ...args),
      list_organization_environments: (...args: Parameters<Operation<"/api/organization/{organization_id}/environments", 'get'>>) => client.GET("/api/organization/{organization_id}/environments", ...args),
      update_environment: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}", 'patch'>>) => client.PATCH("/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}", ...args)
    },
    events: {
      list_events: (...args: Parameters<Operation<"/api/organization/{organization_id}/events", 'get'>>) => client.GET("/api/organization/{organization_id}/events", ...args)
    },
    health: {
      check: (...args: Parameters<Operation<"/health", 'get'>>) => client.GET("/health", ...args)
    },
    internal: {
      provider_credentials: (...args: Parameters<Operation<"/internal/s3-providers/{provider_id}/credentials", 'get'>>) => client.GET("/internal/s3-providers/{provider_id}/credentials", ...args),
      provision_tenant_key: (...args: Parameters<Operation<"/internal/organizations/{organization_id}/transit-key", 'post'>>) => client.POST("/internal/organizations/{organization_id}/transit-key", ...args),
      resolve_access_token: (...args: Parameters<Operation<"/internal/s3-access-tokens/resolve/{access_key}", 'get'>>) => client.GET("/internal/s3-access-tokens/resolve/{access_key}", ...args)
    },
    projects: {
      create_project: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects", 'post'>>) => client.POST("/api/organization/{organization_id}/projects", ...args),
      delete_project: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects/{project_id}", 'delete'>>) => client.DELETE("/api/organization/{organization_id}/projects/{project_id}", ...args),
      get_project: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects/{project_id}", 'get'>>) => client.GET("/api/organization/{organization_id}/projects/{project_id}", ...args),
      get_timeline: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects/{project_id}/timelines/{timeline_id}", 'get'>>) => client.GET("/api/organization/{organization_id}/projects/{project_id}/timelines/{timeline_id}", ...args),
      list_project_timelines: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects/{project_id}/timelines", 'get'>>) => client.GET("/api/organization/{organization_id}/projects/{project_id}/timelines", ...args),
      list_projects: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects", 'get'>>) => client.GET("/api/organization/{organization_id}/projects", ...args)
    },
    regions: {
      list_regions: (...args: Parameters<Operation<"/api/organization/{organization_id}/regions", 'get'>>) => client.GET("/api/organization/{organization_id}/regions", ...args)
    },
    registry: {
      create_access_token: (...args: Parameters<Operation<"/api/organization/{organization_id}/registry/access-tokens", 'post'>>) => client.POST("/api/organization/{organization_id}/registry/access-tokens", ...args),
      create_external_registry: (...args: Parameters<Operation<"/api/organization/{organization_id}/registry/external-registries", 'post'>>) => client.POST("/api/organization/{organization_id}/registry/external-registries", ...args),
      create_repository: (...args: Parameters<Operation<"/api/organization/{organization_id}/registry/repositories", 'post'>>) => client.POST("/api/organization/{organization_id}/registry/repositories", ...args),
      delete_external_registry: (...args: Parameters<Operation<"/api/organization/{organization_id}/registry/external-registries/{registry_id}", 'delete'>>) => client.DELETE("/api/organization/{organization_id}/registry/external-registries/{registry_id}", ...args),
      delete_repository: (...args: Parameters<Operation<"/api/organization/{organization_id}/registry/repositories/{repository_id}", 'delete'>>) => client.DELETE("/api/organization/{organization_id}/registry/repositories/{repository_id}", ...args),
      get_access_token: (...args: Parameters<Operation<"/api/organization/{organization_id}/registry/access-tokens/{token_id}", 'get'>>) => client.GET("/api/organization/{organization_id}/registry/access-tokens/{token_id}", ...args),
      issue_token: (...args: Parameters<Operation<"/api/registry/token", 'get'>>) => client.GET("/api/registry/token", ...args),
      list_access_tokens: (...args: Parameters<Operation<"/api/organization/{organization_id}/registry/access-tokens", 'get'>>) => client.GET("/api/organization/{organization_id}/registry/access-tokens", ...args),
      list_external_registries: (...args: Parameters<Operation<"/api/organization/{organization_id}/registry/external-registries", 'get'>>) => client.GET("/api/organization/{organization_id}/registry/external-registries", ...args),
      list_repositories: (...args: Parameters<Operation<"/api/organization/{organization_id}/registry/repositories", 'get'>>) => client.GET("/api/organization/{organization_id}/registry/repositories", ...args),
      maintenance_status: (...args: Parameters<Operation<"/api/registry/maintenance", 'get'>>) => client.GET("/api/registry/maintenance", ...args),
      rename_external_registry: (...args: Parameters<Operation<"/api/organization/{organization_id}/registry/external-registries/{registry_id}", 'patch'>>) => client.PATCH("/api/organization/{organization_id}/registry/external-registries/{registry_id}", ...args),
      revoke_access_token: (...args: Parameters<Operation<"/api/organization/{organization_id}/registry/access-tokens/{token_id}", 'delete'>>) => client.DELETE("/api/organization/{organization_id}/registry/access-tokens/{token_id}", ...args),
      rotate_external_registry_token: (...args: Parameters<Operation<"/api/organization/{organization_id}/registry/external-registries/{registry_id}/rotate-token", 'post'>>) => client.POST("/api/organization/{organization_id}/registry/external-registries/{registry_id}/rotate-token", ...args),
      update_access_token: (...args: Parameters<Operation<"/api/organization/{organization_id}/registry/access-tokens/{token_id}", 'patch'>>) => client.PATCH("/api/organization/{organization_id}/registry/access-tokens/{token_id}", ...args)
    },
    storage: {
      create_access_token: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens", 'post'>>) => client.POST("/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens", ...args),
      create_bucket: (...args: Parameters<Operation<"/api/organization/{organization_id}/storage/buckets", 'post'>>) => client.POST("/api/organization/{organization_id}/storage/buckets", ...args),
      delete_bucket: (...args: Parameters<Operation<"/api/organization/{organization_id}/storage/buckets/{bucket_id}", 'delete'>>) => client.DELETE("/api/organization/{organization_id}/storage/buckets/{bucket_id}", ...args),
      delete_bucket_objects: (...args: Parameters<Operation<"/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects", 'delete'>>) => client.DELETE("/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects", ...args),
      download_bucket_object: (...args: Parameters<Operation<"/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects/download", 'get'>>) => client.GET("/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects/download", { ...args[0], parseAs: "blob" }),
      get_access_token: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}", 'get'>>) => client.GET("/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}", ...args),
      list_access_tokens: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens", 'get'>>) => client.GET("/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens", ...args),
      list_bucket_objects: (...args: Parameters<Operation<"/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects", 'get'>>) => client.GET("/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects", ...args),
      list_buckets: (...args: Parameters<Operation<"/api/organization/{organization_id}/storage/buckets", 'get'>>) => client.GET("/api/organization/{organization_id}/storage/buckets", ...args),
      revoke_access_token: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}", 'delete'>>) => client.DELETE("/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}", ...args),
      update_access_token: (...args: Parameters<Operation<"/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}", 'patch'>>) => client.PATCH("/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}", ...args)
    }
  } as Sdk
}
