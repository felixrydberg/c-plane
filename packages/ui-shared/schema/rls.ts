import { pgRole } from "drizzle-orm/pg-core";

export const app_identity = pgRole("app_identity").existing();
export const app_tenant = pgRole("app_tenant").existing();
export const app_admin = pgRole("app_admin").existing();
