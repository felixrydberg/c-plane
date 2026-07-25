import { drizzle } from "drizzle-orm/postgres-js"
import { betterAuth } from "better-auth/minimal"
import { admin, twoFactor, haveIBeenPwned, lastLoginMethod } from "better-auth/plugins"
import { passkey } from "@better-auth/passkey"
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { createClient } from "redis"
import * as schema from "../schema";
import { and, eq } from "drizzle-orm";
import { getIdentityDb } from "./db";
import { getPasskeyRegistrationContextSecret, parsePasskeyRegistrationContext } from "./passkey-registration-context";

const { NUXT_REDIS_URL, NUXT_DATABASE_URL, NUXT_AUTH_BASE_URL } = process.env;
if (!NUXT_DATABASE_URL) {
  throw new Error("Database connection string is not defined")
}

if (!NUXT_REDIS_URL) {
  throw new Error("Redis connection string is not defined")
}

const authBaseURL = NUXT_AUTH_BASE_URL ?? "http://localhost:3000"
if (process.env.NODE_ENV === "production" && (!NUXT_AUTH_BASE_URL || new URL(authBaseURL).protocol !== "https:")) {
  throw new Error("NUXT_AUTH_BASE_URL must be set to a public HTTPS origin in production")
}

export const redis = createClient({
  url: NUXT_REDIS_URL
})

redis.on("error", (error) => {
  console.error("Redis client error:", error)
})

await redis.connect()

const db = drizzle(NUXT_DATABASE_URL, { schema })
export const getAuthDb = () => db

const passkeyRegistrationContextSecret = getPasskeyRegistrationContextSecret()

export const auth = betterAuth({
  appName: "C-Plane",
  baseURL: authBaseURL,
  secret: passkeyRegistrationContextSecret,
  trustedOrigins: process.env.NODE_ENV === "production" ? [authBaseURL] : [authBaseURL, "http://ui:3000"],
  database: drizzleAdapter(db, {
    provider: "pg",
    schema,
  }),
  databaseHooks: {
    user: {
      create: {
        before: async (user) => ({
          data: { ...user, name: typeof user.name === "string" ? user.name.trim() : "" },
        }),
      },
    },
  },
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
        const identityDb = getIdentityDb();
        const ownedOrganizations = await identityDb
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
      console.error("sendResetPassword is not implemented. User:", user, "URL:", url);
    },
  },
  emailVerification: {
    sendVerificationEmail: async ({ user, url, token: _token }) => {
      console.error("sendVerificationEmail is not implemented. User:", user, "URL:", url);
    }
  },
  socialProviders: {},
  plugins: [
    admin(),
    twoFactor(),
    passkey({
      registration: {
        requireSession: false,
        resolveUser: async ({ ctx, context }) => {
          const { name, email } = await parsePasskeyRegistrationContext(context, ctx.context.secret)
          if (await ctx.context.internalAdapter.findUserByEmail(email)) {
            throw new Error("An account already exists for this email")
          }

          const id = crypto.randomUUID()
          return { id, name }
        },
        afterVerification: async ({ ctx, user, context }) => {
          const { name, email } = await parsePasskeyRegistrationContext(context, ctx.context.secret)
          if (await ctx.context.internalAdapter.findUserByEmail(email)) {
            throw new Error("An account already exists for this email")
          }

          const authUser = await ctx.context.internalAdapter.createUser({
            id: user.id,
            name,
            email,
            emailVerified: false,
          })
          const session = await ctx.context.internalAdapter.createSession(user.id)
          if (!session || !authUser) throw new Error("Could not create passkey session")

          await ctx.setSignedCookie(
            ctx.context.authCookies.sessionToken.name,
            session.token,
            ctx.context.secret,
            {
              ...ctx.context.authCookies.sessionToken.attributes,
              maxAge: ctx.context.sessionConfig.expiresIn,
            },
          )
          ctx.context.setNewSession({ session, user: authUser })
        },
      },
    }),
    lastLoginMethod(),
    haveIBeenPwned({
      customPasswordCompromisedMessage: "This password has been compromised in a data breach, please choose a different one.",
    })
  ]
})
