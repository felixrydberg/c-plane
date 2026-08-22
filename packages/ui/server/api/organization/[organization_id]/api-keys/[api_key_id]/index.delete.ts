import { and, eq } from "drizzle-orm";
import { api_keys } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";

export default defineEventHandler(async (event) => {
  const params = getRouterParams(event);
  const organization_id = params.organization_id as string;
  const api_key_id = params.api_key_id as string;

  await requireScope(event, "api-key:manage", organization_id);
  const deleted = await withTenantDb([organization_id], async (tx) => {
    const [deleted] = await tx
      .delete(api_keys)
      .where(
        and(
          eq(api_keys.id, api_key_id),
          eq(api_keys.organization_id, organization_id)
        )
      )
      .returning();

    if (deleted) {
      await logEvent(organization_id, "api-key:revoked", {
        id: deleted.id,
        organization_id,
        name: deleted.name,
        created_at: deleted.created_at,
      }, false, {}, tx);
    }

    return deleted;
  });

  if (!deleted) {
    throw createError({
      statusCode: 404,
      statusMessage: "API key not found",
    });
  }
});
