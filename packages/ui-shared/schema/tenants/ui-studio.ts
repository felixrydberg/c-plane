import { relations, sql } from "drizzle-orm";
import {
  pgTable,
  text,
  timestamp,
  boolean,
  uuid,
  index,
} from "drizzle-orm/pg-core";

export const studio_user = pgTable("studio_user", {
  id: uuid("id")
    .default(sql`pg_catalog.gen_random_uuid()`)
    .primaryKey(),
  name: text("name").notNull(),
  email: text("email").notNull().unique(),
  emailVerified: boolean("email_verified").default(false).notNull(),
  image: text("image"),
  createdAt: timestamp("created_at").defaultNow().notNull(),
  updatedAt: timestamp("updated_at")
    .defaultNow()
    .$onUpdate(() => /* @__PURE__ */ new Date())
    .notNull(),
  role: text("role"),
  banned: boolean("banned").default(false),
  banReason: text("ban_reason"),
  banExpires: timestamp("ban_expires"),
  twoFactorEnabled: boolean("two_factor_enabled").default(false),
});

export const studio_account = pgTable(
  "studio_account",
  {
    id: uuid("id")
      .default(sql`pg_catalog.gen_random_uuid()`)
      .primaryKey(),
    accountId: text("account_id").notNull(),
    providerId: text("provider_id").notNull(),
    userId: uuid("user_id")
      .notNull()
      .references(() => studio_user.id, { onDelete: "cascade" }),
    accessToken: text("access_token"),
    refreshToken: text("refresh_token"),
    idToken: text("id_token"),
    accessTokenExpiresAt: timestamp("access_token_expires_at"),
    refreshTokenExpiresAt: timestamp("refresh_token_expires_at"),
    scope: text("scope"),
    password: text("password"),
    createdAt: timestamp("created_at").defaultNow().notNull(),
    updatedAt: timestamp("updated_at")
      .$onUpdate(() => /* @__PURE__ */ new Date())
      .notNull(),
  },
  (table) => [index("studio_account_userId_idx").on(table.userId)],
);

export const studio_verification = pgTable(
  "studio_verification",
  {
    id: uuid("id")
      .default(sql`pg_catalog.gen_random_uuid()`)
      .primaryKey(),
    identifier: text("identifier").notNull(),
    value: text("value").notNull(),
    expiresAt: timestamp("expires_at").notNull(),
    createdAt: timestamp("created_at").defaultNow().notNull(),
    updatedAt: timestamp("updated_at")
      .defaultNow()
      .$onUpdate(() => /* @__PURE__ */ new Date())
      .notNull(),
  },
  (table) => [
    index("studio_verification_identifier_idx").on(table.identifier),
  ],
);

export const studio_two_factor = pgTable(
  "studio_two_factor",
  {
    id: uuid("id")
      .default(sql`pg_catalog.gen_random_uuid()`)
      .primaryKey(),
    secret: text("secret").notNull(),
    backupCodes: text("backup_codes").notNull(),
    userId: uuid("user_id")
      .notNull()
      .references(() => studio_user.id, { onDelete: "cascade" }),
  },
  (table) => [
    index("studio_two_factor_secret_idx").on(table.secret),
    index("studio_two_factor_userId_idx").on(table.userId),
  ],
);

export const studio_userRelations = relations(studio_user, ({ many }) => ({
  studio_accounts: many(studio_account),
  studio_two_factors: many(studio_two_factor),
}));

export const studio_accountRelations = relations(studio_account, ({ one }) => ({
  studio_user: one(studio_user, {
    fields: [studio_account.userId],
    references: [studio_user.id],
  }),
}));

export const studio_two_factorRelations = relations(
  studio_two_factor,
  ({ one }) => ({
    studio_user: one(studio_user, {
      fields: [studio_two_factor.userId],
      references: [studio_user.id],
    }),
  }),
);
