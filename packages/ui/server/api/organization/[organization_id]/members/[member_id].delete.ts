import { organization_member } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { eq, and, count } from "drizzle-orm";
import { getOrganizationMembership, assertAllowed } from "~~/server/utils/authorization";
import { denyRemoveMember } from "~~/server/utils/permissions";

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
    const [target] = await tx
      .select()
      .from(organization_member)
      .where(
        and(
          eq(organization_member.id, member_id),
          eq(organization_member.organization_id, membership.organization_id)
        )
      )
      .limit(1);

    if (!target) {
      return null;
    }

    const ownerRows = await tx
      .select({ value: count() })
      .from(organization_member)
      .where(and(
        eq(organization_member.organization_id, membership.organization_id),
        eq(organization_member.role, "owner"),
      ));

    assertAllowed(denyRemoveMember(membership, target, {
      isSelf: target.user_id === membership.user_id,
      ownerCount: Number(ownerRows[0]?.value ?? 0),
    }));

    const [deletedMember] = await tx
      .delete(organization_member)
      .where(eq(organization_member.id, target.id))
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
