import { and, eq } from "drizzle-orm";
import { z } from "zod";

import { s3_provider } from "~~/server/schema";
import { withAdminDb } from "~~/server/utils/db";
import { requireAdmin } from "~~/server/utils/authorization";
import { serializeProvider } from "~~/server/utils/s3-providers";

const listProvidersQuerySchema = z.object({
  is_active: z.enum(["true", "false"]).optional(),
});

export default defineEventHandler(async (event) => {
  await requireAdmin(event);

  const parsed = listProvidersQuerySchema.safeParse(getQuery(event));
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: parsed.error.issues[0]?.message || "Invalid query params" });
  }

  const { is_active } = parsed.data;

  const predicates = [];
  if (is_active) {
    predicates.push(eq(s3_provider.is_active, is_active === "true"));
  }

  const rows = await withAdminDb((db) => {
    return db
      .select({
        id: s3_provider.id,
        provider_type: s3_provider.provider_type,
        endpoint_url: s3_provider.endpoint_url,
        provider_region: s3_provider.provider_region,
        is_active: s3_provider.is_active,
        created_at: s3_provider.created_at,
        updated_at: s3_provider.updated_at,
      })
      .from(s3_provider)
      .where(predicates.length > 0 ? and(...predicates) : undefined);
  });

  return rows.map(serializeProvider);
});
