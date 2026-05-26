import { pgRole } from "drizzle-orm/pg-core";
import { type AnyColumn, sql } from "drizzle-orm";

export const app_identity = pgRole("app_identity").existing();
export const app_tenant = pgRole("app_tenant").existing();
export const app_admin = pgRole("app_admin").existing();

export const orgAllowed = (orgIdCol: AnyColumn) =>
  sql`${orgIdCol} = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))`;

export const deploymentAllowed = (deploymentIdCol: AnyColumn) =>
  sql`${deploymentIdCol} IN (SELECT id FROM deployments WHERE organization_id = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])))`;

