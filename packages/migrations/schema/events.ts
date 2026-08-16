import { boolean, index, json, pgPolicy, pgTable, text, timestamp, uuid } from "drizzle-orm/pg-core";
import { organization } from "./tenants/organization.ts";
import type { EventType } from "../utils/event-types.ts";
export type { EventType } from "../utils/event-types.ts";
import { app_tenant, orgAllowed } from "./rls.ts";

export const event = pgTable("event", {
  id: uuid("id").primaryKey(),
  organization_id: uuid("organization_id")
  .notNull()
  .references(() => organization.id, { onDelete: "cascade" }),
  type: text("type").$type<EventType>().notNull(),
  payload: json("payload").notNull(),
  system: boolean("system").notNull().default(false),
  project_id: uuid("project_id"),
  actor_id: uuid("actor_id"),
  created_at: timestamp("created_at").notNull().defaultNow(),
}, (table) => [
  index("event_organization_id_idx").on(table.organization_id),
  index("event_type_idx").on(table.type),
  index("event_project_idx").on(table.project_id, table.created_at),
  pgPolicy("event_org_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();
