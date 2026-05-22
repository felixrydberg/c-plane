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

  const deleted = await withAdminDb((db) => {
    return db
      .delete(region)
      .where(eq(region.id, regionId))
      .returning({ id: region.id });
  });

  if (deleted.length === 0) {
    throw createError({ statusCode: 404, statusMessage: "Region not found" });
  }

  event.res.status = 204;
  return null;
});
