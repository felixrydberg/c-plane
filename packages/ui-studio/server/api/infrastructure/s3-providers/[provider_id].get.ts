import { eq } from "drizzle-orm";

import { s3_provider } from "~~/server/schema";
import { db } from "~~/server/utils/auth";
import { requireSession } from "~~/server/utils/authorization";
import { serializeProvider } from "~~/server/utils/s3-providers";

export default defineEventHandler(async (event) => {
  await requireSession(event);

  const providerId = getRouterParam(event, "provider_id");
  if (!providerId) {
    throw createError({ statusCode: 400, statusMessage: "Missing provider_id" });
  }

  const [provider] = await db
    .select()
    .from(s3_provider)
    .where(eq(s3_provider.id, providerId))
    .limit(1);

  if (!provider) {
    throw createError({ statusCode: 404, statusMessage: "Provider config not found" });
  }

  return serializeProvider({
    ...provider,
    has_session_token: Boolean(provider.session_token_encrypted),
    has_secret_access_key: Boolean(provider.secret_access_key_encrypted),
  });
});
