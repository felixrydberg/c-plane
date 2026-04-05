import { and, eq } from "drizzle-orm";
import { z } from "zod";

import { s3_provider } from "~~/server/schema";
import { db } from "~~/server/utils/auth";
import { requireSession } from "~~/server/utils/authorization";
import { serializeProvider } from "~~/server/utils/s3-providers";

const listProvidersQuerySchema = z.object({
  is_active: z.enum(["true", "false"]).optional(),
});

export default defineEventHandler(async (event) => {
  await requireSession(event);

  const parsed = listProvidersQuerySchema.safeParse(getQuery(event));
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: parsed.error.issues[0]?.message || "Invalid query params" });
  }

  const { is_active } = parsed.data;

  const predicates = [];
  if (is_active) {
    predicates.push(eq(s3_provider.is_active, is_active === "true"));
  }

  const rows = await db
    .select({
      id: s3_provider.id,
      provider_type: s3_provider.provider_type,
      endpoint_url: s3_provider.endpoint_url,
      provider_region: s3_provider.provider_region,
      access_key_id: s3_provider.access_key_id,
      is_active: s3_provider.is_active,
      has_session_token: s3_provider.session_token_encrypted,
      has_secret_access_key: s3_provider.secret_access_key_encrypted,
      created_at: s3_provider.created_at,
      updated_at: s3_provider.updated_at,
    })
    .from(s3_provider)
    .where(predicates.length > 0 ? and(...predicates) : undefined);

  return rows.map((row) => serializeProvider({
    ...row,
    has_session_token: Boolean(row.has_session_token),
    has_secret_access_key: Boolean(row.has_secret_access_key),
  }));
});
