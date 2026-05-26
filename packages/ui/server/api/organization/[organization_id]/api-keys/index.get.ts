import { withTenantDb } from "~~/server/utils/db";
import { api_keys } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { getOrganizationMembership } from "~~/server/utils/authorization";

export default defineEventHandler(async (event) => {
  const params = getRouterParams(event);
  const organization_id = params.organization_id as string;

  await getOrganizationMembership(event, organization_id);

  const query = getQuery(event);
  const limit = Math.min(parseInt(query.limit as string) || 50, 100);
  const offset = parseInt(query.offset as string) || 0;

  const { keys, totalResult } = await withTenantDb([organization_id], async (tx) => {
    const keys = await tx
      .select()
      .from(api_keys)
      .where(eq(api_keys.organization_id, organization_id))
      .limit(limit)
      .offset(offset);

    const totalResult = await tx.$count(
      api_keys,
      eq(api_keys.organization_id, organization_id)
    );

    return { keys, totalResult };
  });

  return {
    data: keys,
    pagination: {
      total: totalResult,
      limit,
      offset,
    },
  };
});
