import { boolean, index, json, pgEnum, pgTable, timestamp, uuid } from "drizzle-orm/pg-core";
import { organization } from "../organization/schema";

export const event_types = pgEnum("event_types", [
  "organization:member_added",
  "organization:member_removed",
  "organization:invitation_created",
  "organization:invitation_accepted",
  "organization:invitation_revoked",
  "organization:invitation_declined",
  
  "api-key:created",
  "api-key:revoked",
  "api-key:updated",
  "api-key:rolled",
  
  "verification:created",
  "verification:completed",
]);

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
]);
