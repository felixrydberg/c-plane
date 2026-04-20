import { eq } from "drizzle-orm";
import { z } from "zod";

import { s3_provider, S3_PROVIDER_TYPES } from "~~/server/schema";
import { withAdminDb } from "~~/server/utils/db";
import { requireAdmin } from "~~/server/utils/authorization";
import { serializeProvider } from "~~/server/utils/s3-providers";
import { encryptCredential } from "~~/server/utils/storage-credentials";

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
});

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

  const [updated] = await withAdminDb((db) => {
    return db
      .update(s3_provider)
      .set({
        provider_type: body.provider_type,
        endpoint_url: body.endpoint_url,
        provider_region: body.provider_region,
        access_key_id: body.access_key_id,
        secret_access_key_encrypted: body.secret_access_key
          ? encryptCredential(body.secret_access_key)
          : undefined,
        session_token_encrypted: body.session_token === undefined
          ? undefined
          : body.session_token === null
            ? null
            : encryptCredential(body.session_token),
        is_active: body.is_active,
        updated_at: new Date().toISOString(),
      })
      .where(eq(s3_provider.id, providerId))
      .returning();
  });

  if (!updated) {
    throw createError({ statusCode: 404, statusMessage: "Provider config not found" });
  }

  return serializeProvider({
    ...updated,
    has_session_token: Boolean(updated.session_token_encrypted),
    has_secret_access_key: Boolean(updated.secret_access_key_encrypted),
  });
});
