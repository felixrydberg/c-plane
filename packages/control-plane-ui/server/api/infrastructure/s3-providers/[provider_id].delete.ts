import { eq } from "drizzle-orm";

import { s3_provider } from "~~/server/schema";
import { withAdminDb } from "~~/server/utils/db";
import { requireAdmin } from "~~/server/utils/authorization";

export default defineEventHandler(async (event) => {
  await requireAdmin(event);

  const providerId = getRouterParam(event, "provider_id");
  if (!providerId) {
    throw createError({ statusCode: 400, statusMessage: "Missing provider_id" });
  }

  const [deleted] = await withAdminDb((db) => {
    return db
      .delete(s3_provider)
      .where(eq(s3_provider.id, providerId))
      .returning({ id: s3_provider.id });
  });

  if (!deleted) {
    throw createError({ statusCode: 404, statusMessage: "Provider config not found" });
  }

  event.res.status = 204;
  return null;
});
