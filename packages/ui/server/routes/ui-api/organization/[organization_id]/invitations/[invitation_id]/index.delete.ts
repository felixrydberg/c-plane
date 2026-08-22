import { organization_invitation } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
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

  await withTenantDb([organizationId], async (tx) => {
    const updated = await tx
      .update(organization_invitation)
      .set({ status: "revoked" })
      .where(
        and(
          eq(organization_invitation.id, invitationId),
          eq(organization_invitation.organization_id, organizationId),
        ),
      )
      .returning();

    if (updated[0]) {
      await logEvent(organizationId, "organization:invitation_revoked", {
        id: updated[0].id,
        organization_id: updated[0].organization_id,
        email: updated[0].email,
        role: updated[0].role,
        status: updated[0].status,
        expires_at: updated[0].expires_at,
        inviter_id: updated[0].inviter_id,
        created_at: updated[0].created_at,
      }, false, {}, tx);
    }

    return updated;
  });
});
