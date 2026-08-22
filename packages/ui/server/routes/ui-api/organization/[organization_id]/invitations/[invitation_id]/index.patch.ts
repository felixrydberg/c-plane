import { organization_invitation, organization_member } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { eq, and } from "drizzle-orm";
import { uuidv7 } from "uuidv7";

export default defineEventHandler(async (event) => {
  const session = await requireSession(event);
  const params = getRouterParams(event);
  const body = await readBody<{ action: "accept" | "decline" }>(event);

  const invitationId = params.invitation_id as string;
  const organizationId = params.organization_id as string;
  const action = body.action;

  if (!invitationId) {
    throw createError({
      statusCode: 400,
      statusMessage: "Invitation ID is required",
    });
  }

  if (!action || !["accept", "decline"].includes(action)) {
    throw createError({
      statusCode: 400,
      statusMessage: "Action must be 'accept' or 'decline'",
    });
  }

  const invitations = await withTenantDb([organizationId], async (tx) =>
    tx
      .select()
      .from(organization_invitation)
      .where(
        and(
          eq(organization_invitation.id, invitationId),
          eq(organization_invitation.organization_id, organizationId),
          eq(organization_invitation.email, session.user.email || ""),
        ),
      )
      .limit(1),
  );

  if (!invitations || !invitations[0]) {
    throw createError({
      statusCode: 404,
      statusMessage: "Invitation not found or email does not match",
    });
  }

  const invitation = invitations[0];

  if (invitation.status !== "pending") {
    throw createError({
      statusCode: 400,
      statusMessage: "Invitation has already been processed",
    });
  }

  if (new Date(invitation.expires_at) < new Date()) {
    throw createError({
      statusCode: 400,
      statusMessage: "Invitation has expired",
    });
  }

  if (action === "decline") {
    const [updated] = await withTenantDb([organizationId], async (tx) => {
      const [result] = await tx
        .update(organization_invitation)
        .set({ status: "declined" })
        .where(eq(organization_invitation.id, invitationId))
        .returning();

      if (result) {
        await logEvent(organizationId, "organization:invitation_declined", {
          id: result.id,
          organization_id: result.organization_id,
          email: result.email,
          role: result.role,
          status: result.status,
          expires_at: result.expires_at,
          inviter_id: result.inviter_id,
          created_at: result.created_at,
        }, false, {}, tx);
      }

      return [result];
    });

    if (!updated) {
      throw createError({
        statusCode: 500,
        statusMessage: "Failed to decline invitation",
      });
    }

    return updated;
  } else {
    const [updatedInvitation] = await withTenantDb([organizationId], async (tx) => {
      const updated = await tx
        .update(organization_invitation)
        .set({ status: "accepted" })
        .where(eq(organization_invitation.id, invitationId))
        .returning();

      const organization_member_id = uuidv7();
      await tx.insert(organization_member).values({
        id: organization_member_id,
        organization_id: organizationId,
        user_id: session.user.id,
        role: invitation.role,
      });

      if (updated[0]) {
        await logEvent(organizationId, "organization:invitation_accepted", {
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

    if (!updatedInvitation) {
      throw createError({
        statusCode: 500,
        statusMessage: "Failed to accept invitation",
      });
    }

    return updatedInvitation;
  }
});
