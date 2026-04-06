import { sql } from "drizzle-orm";
import { boolean, index, json, pgEnum, pgPolicy, pgTable, timestamp, uuid } from "drizzle-orm/pg-core";
import { organization } from "../organization/schema";
import { EVENT_TYPE_VALUES } from "../../utils/event-types";
import { app_tenant } from "../rls";

export const event_types = pgEnum("event_types", EVENT_TYPE_VALUES);

export const event = pgTable("event", {
  id: uuid("id").primaryKey(),
  organization_id: uuid("organization_id")
  .notNull()
  .references(() => organization.id, { onDelete: "cascade" }),
  type: event_types("type").notNull(),
  payload: json("payload").notNull(),
  system: boolean("system").notNull().default(false),
  created_at: timestamp("created_at").notNull().defaultNow(),
}, (table) => [
  index("event_organization_id_idx").on(table.organization_id),
  index("event_type_idx").on(table.type),
  pgPolicy("event_org_rls", {
    as: "permissive",
    for: "insert",
    to: app_tenant,
    withCheck: sql`${table.organization_id} = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))`,
  }),
]).enableRLS();
