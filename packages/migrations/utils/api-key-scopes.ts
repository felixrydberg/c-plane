export const API_KEY_SCOPE_VALUES = [
  "region:read",
  "project:read",
  "project:create",
  "project:delete",
  "project:manage",
  "access-token:read",
  "access-token:create",
  "access-token:update",
  "access-token:delete",
  "bucket:read",
  "bucket:create",
  "bucket:delete",
  "timeline:read",
  "event:read",
  "container:read",
  "container:create",
  "container:update",
  "container:delete",
  "database:postgres:read",
  "database:postgres:create",
  "database:postgres:update",
  "database:postgres:delete",
  "database:postgres:manage",
  "registry:read",
  "registry:create",
  "registry:update",
  "registry:delete",
] as const;

export const ORGANIZATION_MANAGEMENT_SCOPE_VALUES = [
  "org:update",
  "member:invite",
  "member:remove",
  "api-key:manage",
] as const;

// Member permissions reuse the API-key vocabulary plus org-management scopes.
// API keys stay resource-scoped and never carry org-management scopes.
export const MEMBER_PERMISSION_SCOPE_VALUES = [
  ...API_KEY_SCOPE_VALUES,
  ...ORGANIZATION_MANAGEMENT_SCOPE_VALUES,
] as const;

export type ApiKeyScope = (typeof API_KEY_SCOPE_VALUES)[number];
export type MemberPermissionScope = (typeof MEMBER_PERMISSION_SCOPE_VALUES)[number];

export const isMemberPermissionScope = (
  value: string,
): value is MemberPermissionScope =>
  (MEMBER_PERMISSION_SCOPE_VALUES as readonly string[]).includes(value);
