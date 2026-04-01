import { organization_member } from "~~/server/schema";
import { db } from "~~/server/utils/auth";
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

  const existingMember = await db
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

  await db
    .delete(organization_member)
    .where(eq(organization_member.id, member_id));

  await logEvent(membership.organization_id, "organization:member_removed", {
    id: existingMember[0].id,
    organization_id: existingMember[0].organization_id,
    user_id: existingMember[0].user_id,
    role: existingMember[0].role,
    created_at: existingMember[0].created_at,
  }, false);
});
