import { organization_invitation, organization_member, user } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { and, eq } from "drizzle-orm";
import { uuidv7 } from "uuidv7";
import { logEvent } from "~~/server/utils/events";
import { requireOwner } from "~~/server/utils/authorization";
import z from "zod";

const invitationSchema = z.object({
  email: z.string().email("A valid email address is required"),
  role: z.enum(["member", "admin", "owner"]),
  organization_id: z.string().uuid("Organization ID must be a valid UUID"),
});

export default defineEventHandler(async (event) => {
  const parsed = invitationSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw createError({
      statusCode: 400,
      statusMessage: parsed.error.issues[0]?.message || "Valid email, role, and organization ID are required",
    });
  }

  const { email, role: inviteRole, organization_id: organizationId } = parsed.data;
  const inviteEmail = email.trim().toLowerCase();

  const membership = await requireOwner(event, organizationId);

  if (membership.organization_id !== organizationId) {
    throw createError({
      statusCode: 403,
      statusMessage: "Organization mismatch",
    });
  }

  const invitationId = uuidv7();
  const expiresAt = new Date(Date.now() + 30 * 24 * 60 * 60 * 1000); // 30 days

  const invitation = await withTenantDb([organizationId], async (tx) => {
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
