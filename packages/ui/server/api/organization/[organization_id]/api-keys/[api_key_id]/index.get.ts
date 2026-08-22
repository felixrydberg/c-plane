import { withTenantDb } from "~~/server/utils/db";
import { api_keys, api_key_scopes } from "~~/server/schema";
import { eq, and } from "drizzle-orm";
import { requireScope } from "~~/server/utils/authorization";

export default defineEventHandler(async (event) => {
  const params = getRouterParams(event);
  const organization_id = params.organization_id as string;
  const api_key_id = params.api_key_id as string;

  await requireScope(event, "api-key:manage", organization_id);
  const { key, scopes } = await withTenantDb([organization_id], async (tx) => {
    const result = await tx
      .select()
      .from(api_keys)
      .where(
        and(
          eq(api_keys.id, api_key_id),
          eq(api_keys.organization_id, organization_id)
        )
      );

    if (result.length === 0) {
      throw createError({
        statusCode: 404,
        statusMessage: "API key not found",
      });
    }

    const scopes = await tx
      .select()
      .from(api_key_scopes)
      .where(eq(api_key_scopes.api_key_id, api_key_id));

    return { key: result[0], scopes };
  });

  return {
    ...key,
    scopes: scopes.map((s) => s.scope),
  };
});
