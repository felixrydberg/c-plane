import { organization_invitation, organization_member, user } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { and, eq } from "drizzle-orm";
import { uuidv7 } from "uuidv7";
import { logEvent } from "~~/server/utils/events";

export default defineEventHandler(async (event) => {
  const organizationId = getRouterParam(event, "organization_id");
  if (!organizationId) {
    throw createError({
      statusCode: 400,
      statusMessage: "Organization ID is required",
    });
  }

  const body = await readBody<{
    email?: string;
    role?: "member" | "admin";
  }>(event) ?? {};

  const { email, role } = body;
  const inviteEmail = typeof email === "string" ? email.trim().toLowerCase() : "";

  if (!email || !inviteEmail) {
    throw createError({
      statusCode: 400,
      statusMessage: "Email is required",
    });
  }

  if (role !== "member" && role !== "admin") {
    throw createError({
      statusCode: 400,
      statusMessage: "Role must be 'member' or 'admin'",
    });
  }

  const membership = await getOrganizationMembership(event, organizationId);
  if (!membership) {
    throw createError({
      statusCode: 403,
      statusMessage: "Membership required",
    });
  }

  const invitationId = uuidv7();
  const expiresAt = new Date(Date.now() + 30 * 24 * 60 * 60 * 1000); // 30 days

  const invitation = await withTenantDb([organizationId], async (tx) => {
    const existingInvitation = await tx
      .select()
      .from(organization_invitation)
      .where(and(
        eq(organization_invitation.organization_id, organizationId),
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
        eq(organization_member.organization_id, organizationId),
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
        organization_id: organizationId,
        email: inviteEmail,
        role,
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

    await logEvent(organizationId, "organization:invitation_created", {
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
