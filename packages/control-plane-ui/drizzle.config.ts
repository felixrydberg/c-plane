import { defineConfig } from "drizzle-kit"

export default defineConfig({
  dialect: "postgresql",
  schema: "./server/schema/index.ts",
  out: "./drizzle",
  dbCredentials: {
    url: "postgresql://pingu:mysecretpassword@localhost:5432/pingu"
  }
})
