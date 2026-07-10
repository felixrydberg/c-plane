import { pgRole } from "drizzle-orm/pg-core";
import { type AnyColumn, sql } from "drizzle-orm";

export const app_tenant = pgRole("app_tenant").existing();

export const orgAllowed = (orgIdCol: AnyColumn) =>
  sql`${orgIdCol} = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))`;

