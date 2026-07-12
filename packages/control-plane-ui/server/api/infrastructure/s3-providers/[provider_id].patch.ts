import { eq } from "drizzle-orm";
import { z } from "zod";

import { s3_provider, S3_PROVIDER_TYPES } from "~~/server/schema";
import { withAdminDb } from "~~/server/utils/db";
import { requireAdmin } from "~~/server/utils/authorization";
import { serializeProvider } from "~~/server/utils/s3-providers";
import { writeS3ProviderCredentials } from "~~/server/utils/openbao";

const updateProviderSchema = z.object({
  provider_type: z.enum(S3_PROVIDER_TYPES).optional(),
  endpoint_url: z.url("endpoint_url must be a valid URL").optional(),
  provider_region: z.string().trim().min(1).nullable().optional(),
  access_key_id: z.string().trim().min(1).optional(),
  secret_access_key: z.string().trim().min(1).optional(),
  session_token: z.string().trim().min(1).nullable().optional(),
  is_active: z.boolean().optional(),
}).refine((value) => Object.keys(value).length > 0, {
  message: "At least one field is required",
}).refine(
  (value) => (value.access_key_id === undefined) === (value.secret_access_key === undefined),
  { message: "access_key_id and secret_access_key must be updated together" },
).refine(
  (value) => value.session_token === undefined || value.access_key_id !== undefined,
  { message: "session_token can only be updated with provider credentials" },
);

export default defineEventHandler(async (event) => {
  await requireAdmin(event);

  const providerId = getRouterParam(event, "provider_id");
  if (!providerId) {
    throw createError({ statusCode: 400, statusMessage: "Missing provider_id" });
  }

  const parsed = updateProviderSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: parsed.error.issues[0]?.message || "Invalid request body" });
  }

  const body = parsed.data;
  const [existing] = await withAdminDb((db) => {
    return db.select({ id: s3_provider.id }).from(s3_provider).where(eq(s3_provider.id, providerId)).limit(1);
  });

  if (!existing) {
    throw createError({ statusCode: 404, statusMessage: "Provider config not found" });
  }

  if (body.access_key_id && body.secret_access_key) {
    await writeS3ProviderCredentials(providerId, {
      access_key_id: body.access_key_id,
      secret_access_key: body.secret_access_key,
      ...(body.session_token ? { session_token: body.session_token } : {}),
    });
  }

  const [updated] = await withAdminDb((db) => {
    return db
      .update(s3_provider)
      .set({
        provider_type: body.provider_type,
        endpoint_url: body.endpoint_url,
        provider_region: body.provider_region,
        is_active: body.is_active,
        updated_at: new Date().toISOString(),
      })
      .where(eq(s3_provider.id, providerId))
      .returning();
  });

  if (!updated) {
    throw createError({ statusCode: 404, statusMessage: "Provider config not found" });
  }

  return serializeProvider(updated);
});
