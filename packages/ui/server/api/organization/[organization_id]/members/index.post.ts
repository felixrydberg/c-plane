import { organization_member, user } from "~~/server/schema";
import { db } from "~~/server/utils/auth";
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

  const existingUser = await db
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

  const existingMember = await db
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

  const newMember = await db
    .insert(organization_member)
    .values({
      id: uuidv7(),
      organization_id: membership.organization_id,
      user_id: targetUser.id,
      role: 'member',
    })
    .returning();

  await logEvent(membership.organization_id, "organization:member_added", {
    id: newMember[0].id,
    organization_id: newMember[0].organization_id,
    user_id: newMember[0].user_id,
    role: newMember[0].role,
    created_at: newMember[0].created_at,
  }, false);

  return newMember[0];
});
