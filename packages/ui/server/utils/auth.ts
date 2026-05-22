import { drizzle } from "drizzle-orm/postgres-js"
import { betterAuth } from "better-auth/minimal"
import { admin, twoFactor, haveIBeenPwned } from "better-auth/plugins"
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { createClient } from "redis"
import * as schema from "../schema";
import { and, eq } from "drizzle-orm";
import { sendEmail } from "./email";
import { createResetPasswordEmailTemplate, createVerifyEmailTemplate } from "./email-templates";

const { NUXT_REDIS_URL, NUXT_DATABASE_URL } = process.env;
if (!NUXT_DATABASE_URL) {
  throw new Error("Database connection string is not defined")
}

if (!NUXT_REDIS_URL) {
  throw new Error("Redis connection string is not defined")
}

export const redis = createClient({
  url: NUXT_REDIS_URL
})

redis.on("error", (error) => {
  console.error("Redis client error:", error)
})

await redis.connect()

console.log("Connecting to database...")

export const db = drizzle(NUXT_DATABASE_URL, { schema })
export const auth = betterAuth({
  appName: "C-Plane",
  baseURL: "http://localhost:3000",
  trustedOrigins: [
    "http://localhost:3000",
    "https://cplane.240284308.xyz"
  ],
  database: drizzleAdapter(db, {
    provider: "pg",
    schema,
  }),
  advanced: {
    database: {
      generateId: "uuid",
    }
  },
  secondaryStorage: {
    get: async key => {
      return redis.get(key)
    },
    set: async (key, value, ttl) => {
      if (ttl) return redis.set(key, value, { EX: ttl })
      else return redis.set(key, value)
    },
    delete: async (key) => {
      await redis.del(key)
    }
  },
  user: {
    deleteUser: {
      enabled: true,
      beforeDelete: async (user) => {
        const ownedOrganizations = await db
          .select({
            name: schema.organization.name,
          })
          .from(schema.organization_member)
          .innerJoin(schema.organization, eq(schema.organization.id, schema.organization_member.organization_id))
          .where(
            and(
              eq(schema.organization_member.user_id, user.id),
              eq(schema.organization_member.role, "owner"),
            ),
          );

        if (ownedOrganizations.length > 0) {
          const organizationNames = ownedOrganizations.map(org => org.name).join(", ");
          throw new Error(organizationNames.length > 0
            ? `Transfer organization ownership before deleting your account: ${organizationNames}`
            : "Transfer organization ownership before deleting your account",
          );
        }
      },
    },
    changeEmail: {
      enabled: true,
    }
  },
  emailAndPassword: {
    enabled: true,
    sendResetPassword: async ({ user, url }) => {
      const template = createResetPasswordEmailTemplate({ url });

      void sendEmail({
        to: user.email,
        subject: template.subject,
        html: template.html
      });
    },
  },
  emailVerification: {
    sendVerificationEmail: async ({ user, url, token: _token }) => {
      const template = createVerifyEmailTemplate({ url });

      void sendEmail({
        to: user.email,
        subject: template.subject,
        html: template.html
      });
    }
  },
  socialProviders: {},
  plugins: [
    admin(),
    twoFactor(),
    haveIBeenPwned({
      customPasswordCompromisedMessage: "This password has been compromised in a data breach, please choose a different one.",
    })
  ]
})
