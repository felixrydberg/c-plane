import { organization_member, user } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { eq } from "drizzle-orm";
import { uuidv7 } from "uuidv7";
import { getOrganizationMembership, assertAllowed } from "~~/server/utils/authorization";
import { denyAddMember } from "~~/server/utils/permissions";

export default defineEventHandler(async (event) => {
  const membership = await getOrganizationMembership(event);
  const { email } = await readBody(event);

  if (!email || typeof email !== 'string') {
    throw createError({
      statusCode: 400,
      statusMessage: 'Email is required',
    });
  }

  assertAllowed(denyAddMember(membership));

  const result = await withTenantDb([membership.organization_id], async (tx) => {
    const [existingUser] = await tx
      .select()
      .from(user)
      .where(eq(user.email, email.toLowerCase()));

    if (!existingUser) {
      throw createError({
        statusCode: 404,
        statusMessage: 'User with this email does not exist',
      });
    }

    const existingMember = await tx
      .select()
      .from(organization_member)
      .where(
        eq(organization_member.organization_id, membership.organization_id) &&
        eq(organization_member.user_id, existingUser.id)
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
        user_id: existingUser.id,
        role: 'member',
      })
      .returning();

    if (newMember[0]) {
      await logEvent(membership.organization_id, "organization:member_added", {
        id: newMember[0].id,
        organization_id: newMember[0].organization_id,
        user_id: newMember[0].user_id,
        role: newMember[0].role,
        created_at: newMember[0].created_at,
      }, false, {}, tx);
    }

    return newMember[0];
  });

  return result;
});
