import { z } from "zod";
import { uuidv7 } from "uuidv7";

import { s3_provider, S3_PROVIDER_TYPES } from "~~/server/schema";
import { withAdminDb } from "~~/server/utils/db";
import { requireAdmin } from "~~/server/utils/authorization";
import { encryptCredential } from "~~/server/utils/storage-credentials";
import { serializeProvider } from "~~/server/utils/s3-providers";

const createProviderSchema = z.object({
  provider_type: z.enum(S3_PROVIDER_TYPES),
  endpoint_url: z.url("endpoint_url must be a valid URL"),
  provider_region: z.string().trim().min(1).optional(),
  access_key_id: z.string().trim().min(1, "access_key_id is required"),
  secret_access_key: z.string().trim().min(1, "secret_access_key is required"),
  session_token: z.string().trim().min(1).optional(),
  is_active: z.boolean().optional(),
});

export default defineEventHandler(async (event) => {
  await requireAdmin(event);

  const parsed = createProviderSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: parsed.error.issues[0]?.message || "Invalid request body" });
  }

  const body = parsed.data;

  const [created] = await withAdminDb((db) => {
    return db.insert(s3_provider).values({
      id: uuidv7(),
      provider_type: body.provider_type,
      endpoint_url: body.endpoint_url,
      provider_region: body.provider_region,
      access_key_id: body.access_key_id,
      secret_access_key_encrypted: encryptCredential(body.secret_access_key),
      session_token_encrypted: body.session_token ? encryptCredential(body.session_token) : null,
      is_active: body.is_active ?? true,
    }).returning();
  });

  if (!created) {
    throw createError({ statusCode: 500, statusMessage: "Failed to create provider configuration" });
  }

  event.res.status = 201;
  return serializeProvider({
    ...created,
    has_session_token: Boolean(created.session_token_encrypted),
    has_secret_access_key: Boolean(created.secret_access_key_encrypted),
  });
});
