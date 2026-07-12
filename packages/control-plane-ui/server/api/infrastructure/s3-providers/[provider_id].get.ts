import { eq } from "drizzle-orm";

import { s3_provider } from "~~/server/schema";
import { withAdminDb } from "~~/server/utils/db";
import { requireAdmin } from "~~/server/utils/authorization";
import { serializeProvider } from "~~/server/utils/s3-providers";

export default defineEventHandler(async (event) => {
  await requireAdmin(event);

  const providerId = getRouterParam(event, "provider_id");
  if (!providerId) {
    throw createError({ statusCode: 400, statusMessage: "Missing provider_id" });
  }

  const [provider] = await withAdminDb((db) => {
    return db
      .select()
      .from(s3_provider)
      .where(eq(s3_provider.id, providerId))
      .limit(1);
  });

  if (!provider) {
    throw createError({ statusCode: 404, statusMessage: "Provider config not found" });
  }

  return serializeProvider(provider);
});
