import { organization_invitation } from "~~/server/schema";
import { db } from "~~/server/utils/auth";
import { eq, and } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const params = getRouterParams(event);
  const invitationId = params.invitation_id as string;
  const organizationId = params.organization_id as string;

  if (!invitationId) {
    throw createError({
      statusCode: 400,
      statusMessage: "Invitation ID is required",
    });
  }

  const membership = await getOrganizationMembership(event);
  if (membership.organization_id !== organizationId) {
    throw createError({
      statusCode: 403,
      statusMessage: "Organization mismatch",
    });
  }

  const invitations = await db
    .select()
    .from(organization_invitation)
    .where(
      and(
        eq(organization_invitation.id, invitationId),
        eq(organization_invitation.organization_id, organizationId),
      ),
    );

  await db
    .update(organization_invitation)
    .set({ status: "revoked" })
    .where(
      and(
        eq(organization_invitation.id, invitationId),
        eq(organization_invitation.organization_id, organizationId),
      ),
    )
    .returning();

  if (invitations.length > 0) {
    await logEvent(organizationId, "organization:invitation_revoked", {
      id: invitations[0].id,
      organization_id: invitations[0].organization_id,
      email: invitations[0].email,
      role: invitations[0].role,
      status: invitations[0].status,
      expires_at: invitations[0].expires_at,
      inviter_id: invitations[0].inviter_id,
      created_at: invitations[0].created_at,
    }, false);
  }
});
