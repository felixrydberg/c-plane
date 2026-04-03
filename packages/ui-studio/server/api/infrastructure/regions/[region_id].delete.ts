import { requireSession } from "~~/server/utils/authorization";
import { db } from "~~/server/utils/auth";
import { region } from "~~/server/schema";
import { eq } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  await requireSession(event);
  const regionId = getRouterParam(event, "region_id");
  if (!regionId) {
    throw createError({ statusCode: 400, statusMessage: "Missing region_id" });
  }

  const deleted = await db
    .delete(region)
    .where(eq(region.id, regionId))
    .returning({ id: region.id });

  if (deleted.length === 0) {
    throw createError({ statusCode: 404, statusMessage: "Region not found" });
  }

  event.res.status = 204;
  return null;
});
