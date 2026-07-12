import { z } from "zod";
import { uuidv7 } from "uuidv7";

import { s3_provider, S3_PROVIDER_TYPES } from "~~/server/schema";
import { withAdminDb } from "~~/server/utils/db";
import { requireAdmin } from "~~/server/utils/authorization";
import { deleteS3ProviderCredentials, writeS3ProviderCredentials } from "~~/server/utils/openbao";
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
  const providerId = uuidv7();

  await writeS3ProviderCredentials(providerId, {
    access_key_id: body.access_key_id,
    secret_access_key: body.secret_access_key,
    ...(body.session_token ? { session_token: body.session_token } : {}),
  });

  let created: typeof s3_provider.$inferSelect | undefined;
  try {
    [created] = await withAdminDb((db) => {
      return db.insert(s3_provider).values({
        id: providerId,
        provider_type: body.provider_type,
        endpoint_url: body.endpoint_url,
        provider_region: body.provider_region,
        is_active: body.is_active ?? true,
      }).returning();
    });
  } catch (error) {
    await deleteS3ProviderCredentials(providerId).catch(() => undefined);
    throw error;
  }

  if (!created) {
    throw createError({ statusCode: 500, statusMessage: "Failed to create provider configuration" });
  }

  event.res.status = 201;
  return serializeProvider(created);
});
