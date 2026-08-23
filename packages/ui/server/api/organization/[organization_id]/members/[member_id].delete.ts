import { organization_member } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { eq, and } from "drizzle-orm";
import { requireOwner } from "~~/server/utils/authorization";

export default defineEventHandler(async (event) => {
  const membership = await requireOwner(event);
  const { member_id } = getRouterParams(event);

  if (!member_id) {
    throw createError({
      statusCode: 400,
      statusMessage: 'Member ID is required',
    });
  }

  const deletedMember = await withTenantDb([membership.organization_id], async (tx) => {
    const owners = await tx
      .select({ id: organization_member.id })
      .from(organization_member)
      .where(
        and(
          eq(organization_member.organization_id, membership.organization_id),
          eq(organization_member.role, "owner")
        )
      )
      .for("update");

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
      return undefined;
    }

    if (target.role === "owner") {
      const ownerCount = owners.length;
      if (ownerCount <= 1) {
        throw createError({
          statusCode: 409,
          statusMessage: "Cannot remove the last owner of an organization",
        });
      }
    }

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
