import { drizzle } from "drizzle-orm/postgres-js"
import { betterAuth } from "better-auth/minimal"
import { admin, twoFactor, haveIBeenPwned } from "better-auth/plugins"
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { createClient } from "redis"
import * as schema from "../schema";
import { sendEmail } from "./email";
import { createResetPasswordEmailTemplate, createVerifyEmailTemplate } from "./email-templates";

const { NUXT_REDIS_URL, NUXT_DATABASE_URL } = process.env;
const db_table_prefix = "cplane_";
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

export const db = drizzle(NUXT_DATABASE_URL, { schema })
export const auth = betterAuth({
  appName: "C-Plane Studio",
  baseURL: "http://localhost:3001",
  trustedOrigins: [
    "http://localhost:3001",
    "https://cplane-studio.240284308.xyz"
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
    modelName: `${db_table_prefix}user`,
    deleteUser: {
      enabled: true,
    },
    changeEmail: {
      enabled: true,
    }
  },
  account: {
    modelName: `${db_table_prefix}account`,
  },
  verification: {
    modelName: `${db_table_prefix}auth_verification`,
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
    twoFactor({
      twoFactorTable: `${db_table_prefix}two_factor`,
    }),
    haveIBeenPwned({
      customPasswordCompromisedMessage: "This password has been compromised in a data breach, please choose a different one.",
    })
  ]
})
