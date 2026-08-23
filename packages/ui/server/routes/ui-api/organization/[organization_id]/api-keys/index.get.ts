import { withTenantDb } from "~~/server/utils/db";
import { api_keys } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { getOrganizationMembership } from "~~/server/utils/authorization";
import { pagination } from "~~/server/utils/pagination";

export default defineEventHandler(async (event) => {
  const params = getRouterParams(event);
  const organization_id = params.organization_id as string;

  await getOrganizationMembership(event, organization_id);

  const query = getQuery(event);
  const { limit, offset } = pagination(query);

  const { keys, totalResult } = await withTenantDb([organization_id], async (tx) => {
    const keys = await tx
      .select({
        id: api_keys.id,
        name: api_keys.name,
        created_at: api_keys.created_at,
        expires_at: api_keys.expires_at,
        allowed_ips: api_keys.allowed_ips,
      })
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
