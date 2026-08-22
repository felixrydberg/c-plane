import { organization_invitation, organization_member, user } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { and, eq } from "drizzle-orm";
import { uuidv7 } from "uuidv7";
import { logEvent } from "~~/server/utils/events";
import { requireOwner } from "~~/server/utils/authorization";

export default defineEventHandler(async (event) => {
  const body = await readBody<{
    email: string;
    role: "member" | "admin";
    organization_id: string;
  }>(event);

  const { email, role, organization_id } = body;
  const inviteEmail = email.trim().toLowerCase();

  const validRoles = ["member", "admin", "owner"] as const;
  const inviteRole = validRoles.includes(role as (typeof validRoles)[number])
    ? (role as (typeof validRoles)[number])
    : null;

  if (!email || !inviteEmail || !inviteRole) {
    throw createError({
      statusCode: 400,
      statusMessage: "Valid email and role (member | admin | owner) are required",
    });
  }

  const membership = await requireOwner(event, organization_id);

  if (membership.organization_id !== organization_id) {
    throw createError({
      statusCode: 403,
      statusMessage: "Organization mismatch",
    });
  }

  const invitationId = uuidv7();
  const expiresAt = new Date(Date.now() + 30 * 24 * 60 * 60 * 1000); // 30 days

  const invitation = await withTenantDb([organization_id], async (tx) => {
    const existingInvitation = await tx
      .select()
      .from(organization_invitation)
      .where(and(
        eq(organization_invitation.email, inviteEmail),
        eq(organization_invitation.status, "pending"),
      ))
      .limit(1);

    if (existingInvitation.length > 0) {
      throw createError({
        statusCode: 409,
        statusMessage: "Invitation already exists for this email",
      });
    }

    const existingMember = await tx
      .select()
      .from(organization_member)
      .innerJoin(user, eq(organization_member.user_id, user.id))
      .where(and(
        eq(organization_member.organization_id, organization_id),
        eq(user.email, inviteEmail),
      ))
      .limit(1);

    if (existingMember.length > 0) {
      throw createError({
        statusCode: 409,
        statusMessage: "User is already a member of the organization",
      });
    }

    const [invitation] = await tx
      .insert(organization_invitation)
      .values({
        id: invitationId,
        organization_id: organization_id,
        email: inviteEmail,
        role: inviteRole,
        status: "pending",
        expires_at: expiresAt,
        inviter_id: membership.user_id,
      })
      .returning();

    if (!invitation) {
      throw createError({
        statusCode: 500,
        statusMessage: "Failed to create invitation",
      });
    }

    await logEvent(organization_id, "organization:invitation_created", {
      id: invitation.id,
      organization_id: invitation.organization_id,
      email: invitation.email,
      role: invitation.role,
      status: invitation.status,
      expires_at: invitation.expires_at,
      inviter_id: invitation.inviter_id,
      created_at: invitation.created_at,
    }, false, {}, tx);

    return invitation;
  });

  return invitation;
});
