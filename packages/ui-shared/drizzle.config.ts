import { defineConfig } from "drizzle-kit"

export default defineConfig({
  dialect: "postgresql",
  schema: "./schema/index.ts",
  out: "./drizzle",
  dbCredentials: {
    url: "postgresql://cplane:mysecretpassword123@localhost:5432/cplane"
  }
})
