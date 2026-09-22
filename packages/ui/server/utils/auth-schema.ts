import { betterAuth } from "better-auth/minimal";
import { admin, twoFactor, haveIBeenPwned, lastLoginMethod } from "better-auth/plugins";
import { passkey } from "@better-auth/passkey";

// Keep this config free of runtime services so the migration CLI can inspect
// Better Auth's schema without database or Redis environment variables.
export default betterAuth({
  appName: "C-Plane",
  basePath: "/ui-api/auth",
  advanced: {
    database: {
      generateId: "uuid",
    },
  },
  plugins: [admin(), twoFactor(), passkey(), lastLoginMethod(), haveIBeenPwned()],
});
