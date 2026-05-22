import { requireAdmin } from "~~/server/utils/authorization";
import { withAdminDb } from "~~/server/utils/db";
import { region } from "~~/server/schema";

export default defineEventHandler(async (event) => {
  await requireAdmin(event);
  return withAdminDb((db) => db.select().from(region));
});
