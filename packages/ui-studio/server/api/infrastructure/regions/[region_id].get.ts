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

  const found = await db.query.region.findFirst({
    where: eq(region.id, regionId),
  });

  if (!found) {
    throw createError({ statusCode: 404, statusMessage: "Region not found" });
  }

  return found;
});
