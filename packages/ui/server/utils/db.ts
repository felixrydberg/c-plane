import { eq, sql } from "drizzle-orm";
import { drizzle } from "drizzle-orm/postgres-js";
import { createError } from "h3";

import * as schema from "../schema";

const {
  NUXT_IDENTITY_DATABASE_URL,
  NUXT_TENANT_DATABASE_URL,
} = process.env;

if (!NUXT_IDENTITY_DATABASE_URL) {
  throw new Error("Identity database connection string is not defined");
}

if (!NUXT_TENANT_DATABASE_URL) {
  throw new Error("Tenant database connection string is not defined");
}

const identityDb = drizzle(NUXT_IDENTITY_DATABASE_URL, { schema });
const tenantDb = drizzle(NUXT_TENANT_DATABASE_URL, { schema });

type TenantTransaction = Parameters<
  Parameters<typeof tenantDb.transaction>[0]
>[0];

const toPgUuidArrayLiteral = (organizationIds: string[]) => {
  if (organizationIds.length === 0) {
    return "{}";
  }

  return `{${organizationIds.join(",")}}`;
};

export const getIdentityDb = () => identityDb;
export const getTenantDb = () => tenantDb;

export async function activeOrganizationScope(userId: string, nextOrganizationId: string) {
  const [current] = await identityDb
    .select({ organization_id: schema.active_organization.organization_id })
    .from(schema.active_organization)
    .where(eq(schema.active_organization.user_id, userId))
    .limit(1);

  return [...new Set([nextOrganizationId, current?.organization_id].filter(Boolean))] as string[];
}

export async function withTenantDb<T>(
  allowedOrganizations: string[],
  fn: (tx: TenantTransaction) => Promise<T>,
): Promise<T> {
  if (allowedOrganizations.length === 0) {
    throw createError({
      statusCode: 403,
      statusMessage: "No organization access",
    });
  }

  const allowedOrganizationsLiteral = toPgUuidArrayLiteral(allowedOrganizations);

  return tenantDb.transaction(async (tx) => {
    await tx.execute(
      sql`select set_config('app.allowed_organizations', ${allowedOrganizationsLiteral}, true)`,
    );

    return fn(tx);
  });
}
