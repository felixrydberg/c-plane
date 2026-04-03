import { index, pgTable, text, timestamp, uniqueIndex, uuid } from "drizzle-orm/pg-core";
import { region } from "../regions/schema";

export const region_capability = pgTable("region_capabilities", {
  id: uuid("id").primaryKey(),
  region_id: uuid("region_id")
    .notNull()
    .references(() => region.id, { onDelete: "cascade" }),
  scope: text("scope").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("region_capabilities_region_id_scope_uidx").on(table.region_id, table.scope),
  index("region_capabilities_scope_idx").on(table.scope),
]);
