import { sql } from "drizzle-orm";
import { drizzle } from "drizzle-orm/postgres-js";
import { createError } from "h3";

import * as schema from "../schema";

const {
  NUXT_DATABASE_URL,
  NUXT_IDENTITY_DATABASE_URL,
  NUXT_TENANT_DATABASE_URL,
  NUXT_ADMIN_DATABASE_URL,
} = process.env;

if (!NUXT_DATABASE_URL) {
  throw new Error("Database connection string is not defined");
}

const identityDbUrl = NUXT_IDENTITY_DATABASE_URL ?? NUXT_DATABASE_URL;
const tenantDbUrl = NUXT_TENANT_DATABASE_URL ?? NUXT_DATABASE_URL;
const adminDbUrl = NUXT_ADMIN_DATABASE_URL ?? NUXT_DATABASE_URL;

const identityDb = drizzle(identityDbUrl, { schema });
const tenantDb = drizzle(tenantDbUrl, { schema });
const adminDb = drizzle(adminDbUrl, { schema });

type TenantTransaction = Parameters<
  Parameters<typeof tenantDb.transaction>[0]
>[0];
type AdminTransaction = Parameters<
  Parameters<typeof adminDb.transaction>[0]
>[0];

const toPgUuidArrayLiteral = (organizationIds: string[]) => {
  if (organizationIds.length === 0) {
    return "{}";
  }

  return `{${organizationIds.join(",")}}`;
};

export const getIdentityDb = () => identityDb;
export const getTenantDb = () => tenantDb;
export const getAdminDb = () => adminDb;

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

export const withAdminDb = <T>(
  fn: (tx: AdminTransaction) => Promise<T>,
): Promise<T> => {
  return adminDb.transaction((tx) => fn(tx));
};
