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

  const deletedMember = await withTenantDb([membership.organization_id], async (tx) => {
    const [deletedMember] = await tx
      .delete(organization_member)
      .where(
        and(
          eq(organization_member.id, member_id),
          eq(organization_member.organization_id, membership.organization_id)
        )
      )
      .returning();

    if (deletedMember) {
      await logEvent(membership.organization_id, "organization:member_removed", {
        id: deletedMember.id,
        organization_id: deletedMember.organization_id,
        user_id: deletedMember.user_id,
        role: deletedMember.role,
        created_at: deletedMember.created_at,
      }, false, {}, tx);
    }

    return deletedMember;
  });

  if (!deletedMember) {
    throw createError({
      statusCode: 404,
      statusMessage: "Member not found",
    });
  }
});
