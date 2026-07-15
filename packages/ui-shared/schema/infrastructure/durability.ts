import { boolean, index, pgEnum, pgTable, text, timestamp, uuid } from "drizzle-orm/pg-core";

export const S3_PROVIDER_TYPES = ["aws_s3", "cloudflare_r2"] as const;
export const s3_provider_type = pgEnum("s3_provider_type", S3_PROVIDER_TYPES);

export const s3_provider = pgTable("s3_providers", {
  id: uuid("id").primaryKey(),
  provider_type: s3_provider_type("provider_type").notNull(),
  endpoint_url: text("endpoint_url").notNull(),
  provider_region: text("provider_region").notNull(),
  is_active: boolean("is_active").notNull().default(true),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  index("s3_providers_provider_type_idx").on(table.provider_type),
  index("s3_providers_is_active_idx").on(table.is_active),
]).enableRLS();
