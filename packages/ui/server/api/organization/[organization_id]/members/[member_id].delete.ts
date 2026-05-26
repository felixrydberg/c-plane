import { organization_member } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { eq, and } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const membership = await getOrganizationMembership(event);
  const { member_id } = getRouterParams(event);

  if (!member_id) {
    throw createError({
      statusCode: 400,
      statusMessage: 'Member ID is required',
    });
  }

  const result = await withTenantDb([membership.organization_id], async (tx) => {
    const existingMember = await tx
      .select()
      .from(organization_member)
      .where(
        and(
          eq(organization_member.id, member_id),
          eq(organization_member.organization_id, membership.organization_id)
        )
      );

    if (existingMember.length === 0) {
      throw createError({
        statusCode: 404,
        statusMessage: 'Member not found',
      });
    }

    await tx
      .delete(organization_member)
      .where(eq(organization_member.id, member_id));

    return existingMember[0];
  });

  await logEvent(membership.organization_id, "organization:member_removed", {
    id: result.id,
    organization_id: result.organization_id,
    user_id: result.user_id,
    role: result.role,
    created_at: result.created_at,
  }, false);
});
