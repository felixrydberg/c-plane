import { boolean, index, json, pgEnum, pgPolicy, pgTable, timestamp, uuid } from "drizzle-orm/pg-core";
import { organization } from "./tenants/organization";
import { EVENT_TYPE_VALUES } from "../utils/event-types";
import { app_tenant, orgAllowed } from "./rls";

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
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();
