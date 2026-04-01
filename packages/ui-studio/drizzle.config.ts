import { defineConfig } from "drizzle-kit"

export default defineConfig({
  dialect: "postgresql",
  schema: "./server/schema/index.ts",
  out: "./drizzle",
  dbCredentials: {
    url: "postgresql://pingu:mysecretpassword123@localhost:5432/pingu"
  }
})
