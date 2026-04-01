import { db } from "~~/server/utils/auth";
import { api_keys, api_key_scopes } from "~~/server/schema";
import { eq, and } from "drizzle-orm";
import { getOrganizationMembership } from "~~/server/utils/authorization";

export default defineEventHandler(async (event) => {
  const params = getRouterParams(event);
  const organization_id = params.organization_id as string;
  const api_key_id = params.api_key_id as string;

  await getOrganizationMembership(event, organization_id);
  const result = await db
    .select()
    .from(api_keys)
    .where(
      and(
        eq(api_keys.id, api_key_id),
        eq(api_keys.organization_id, organization_id)
      )
    );

  const key = result && result.length > 0 ? result[0] : null;
  if (!key) {
    throw createError({
      statusCode: 404,
      statusMessage: "API key not found",
    });
  }

  const scopes = await db
    .select()
    .from(api_key_scopes)
    .where(eq(api_key_scopes.api_key_id, api_key_id));

  return {
    ...key,
    scopes: scopes.map((s) => s.scope),
  };
});
