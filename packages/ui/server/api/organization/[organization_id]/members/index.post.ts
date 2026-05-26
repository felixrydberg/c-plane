import { organization_member, user } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { eq } from "drizzle-orm";
import { uuidv7 } from "uuidv7";

export default defineEventHandler(async (event) => {
  const membership = await getOrganizationMembership(event);
  const { email } = await readBody(event);

  if (!email || typeof email !== 'string') {
    throw createError({
      statusCode: 400,
      statusMessage: 'Email is required',
    });
  }

  const result = await withTenantDb([membership.organization_id], async (tx) => {
    const existingUser = await tx
      .select()
      .from(user)
      .where(eq(user.email, email.toLowerCase()));

    if (existingUser.length === 0) {
      throw createError({
        statusCode: 404,
        statusMessage: 'User with this email does not exist',
      });
    }

    const targetUser = existingUser[0];

    const existingMember = await tx
      .select()
      .from(organization_member)
      .where(
        eq(organization_member.organization_id, membership.organization_id) &&
        eq(organization_member.user_id, targetUser.id)
      );

    if (existingMember.length > 0) {
      throw createError({
        statusCode: 409,
        statusMessage: 'User is already a member of this organization',
      });
    }

    const newMember = await tx
      .insert(organization_member)
      .values({
        id: uuidv7(),
        organization_id: membership.organization_id,
        user_id: targetUser.id,
        role: 'member',
      })
      .returning();

    return newMember[0];
  });

  await logEvent(membership.organization_id, "organization:member_added", {
    id: result.id,
    organization_id: result.organization_id,
    user_id: result.user_id,
    role: result.role,
    created_at: result.created_at,
  }, false);

  return result;
});
