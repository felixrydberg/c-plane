import { requireAdmin } from "~~/server/utils/authorization";
import { withAdminDb } from "~~/server/utils/db";
import { region } from "~~/server/schema";
import { eq } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  await requireAdmin(event);
  const regionId = getRouterParam(event, "region_id");
  if (!regionId) {
    throw createError({ statusCode: 400, statusMessage: "Missing region_id" });
  }

  const found = await withAdminDb((db) => db.query.region.findFirst({
    where: eq(region.id, regionId),
  }));

  if (!found) {
    throw createError({ statusCode: 404, statusMessage: "Region not found" });
  }

  return found;
});
