import { organization_invitation, organization_member, user, organization } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { and, eq } from "drizzle-orm";
import { uuidv7 } from "uuidv7";
import { logEvent } from "~~/server/utils/events";

export default defineEventHandler(async (event) => {
  const body = await readBody<{
    email: string;
    role: "member" | "admin";
    organization_id: string;
  }>(event);

  const { email, role, organization_id } = body;
  const inviteEmail = email.trim().toLowerCase();

  if (!email || !inviteEmail) {
    throw createError({
      statusCode: 400,
      statusMessage: "Email is required",
    });
  }

  const membership = await getOrganizationMembership(event);
  if (!membership) {
    throw createError({
      statusCode: 403,
      statusMessage: "Membership required",
    });
  }

  if (membership.organization_id !== organization_id) {
    throw createError({
      statusCode: 403,
      statusMessage: "Organization mismatch",
    });
  }

  const invitationId = uuidv7();
  const expiresAt = new Date(Date.now() + 30 * 24 * 60 * 60 * 1000); // 30 days

  const { invitation, orgName, inviterName, inviteAction, inviteUrl } = await withTenantDb([organization_id], async (tx) => {
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

    const [organizationRecord] = await tx
      .select({
        id: organization.id,
        name: organization.name,
      })
      .from(organization)
      .where(eq(organization.id, organization_id))
      .limit(1);

    if (!organizationRecord) {
      throw createError({
        statusCode: 404,
        statusMessage: "Organization not found",
      });
    }

    const [inviterRecord] = await tx
      .select({
        id: user.id,
        name: user.name,
      })
      .from(user)
      .where(eq(user.id, membership.user_id))
      .limit(1);

    const [invitedUser] = await tx
      .select({ id: user.id })
      .from(user)
      .where(eq(user.email, inviteEmail))
      .limit(1);

    const inviteAction: "signin" | "signup" = invitedUser ? "signin" : "signup";
    const authPath = inviteAction === "signin" ? "/auth/signin" : "/auth/signup";
    const requestUrl = getRequestURL(event);
    const inviteUrl = new URL(authPath, requestUrl.origin);
    inviteUrl.searchParams.set("email", inviteEmail);
    inviteUrl.searchParams.set("redirectTo", `/api/user/invitations/${invitationId}/accept`);

    const [invitation] = await tx
      .insert(organization_invitation)
      .values({
        id: invitationId,
        organization_id: organization_id,
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

    return {
      invitation,
      orgName: organizationRecord.name,
      inviterName: inviterRecord?.name,
      inviteAction,
      inviteUrl: inviteUrl.toString(),
    };
  });

  return invitation;
});
