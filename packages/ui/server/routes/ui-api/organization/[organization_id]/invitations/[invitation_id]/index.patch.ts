import { organization_invitation, organization_member } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { eq, and } from "drizzle-orm";
import { uuidv7 } from "uuidv7";

export default defineEventHandler(async (event) => {
  const session = await requireSession(event);
  const { invitation_id: invitationId, organization_id: organizationId } = getRouterParams(event);
  const body = await readBody<{ action?: "accept" | "decline" }>(event);
  const action = body?.action;

  if (!invitationId || !organizationId) {
    throw createError({ statusCode: 400, statusMessage: "Invitation and organization IDs are required" });
  }
  if (action !== "accept" && action !== "decline") {
    throw createError({ statusCode: 400, statusMessage: "Action must be 'accept' or 'decline'" });
  }

  const nextStatus = action === "accept" ? "accepted" : "declined";
  const eventName = action === "accept"
    ? "organization:invitation_accepted"
    : "organization:invitation_declined";

  return withTenantDb([organizationId], async (tx) => {
    const where = and(
      eq(organization_invitation.id, invitationId),
      eq(organization_invitation.organization_id, organizationId),
      eq(organization_invitation.email, session.user.email || ""),
    );
    const [invitation] = await tx.select().from(organization_invitation).where(where).limit(1);

    if (!invitation) {
      throw createError({ statusCode: 404, statusMessage: "Invitation not found or email does not match" });
    }
    if (invitation.status !== "pending" && invitation.status !== nextStatus) {
      throw createError({ statusCode: 409, statusMessage: "Invitation has already been processed" });
    }
    if (invitation.status === "pending" && new Date(invitation.expires_at) < new Date()) {
      throw createError({ statusCode: 400, statusMessage: "Invitation has expired" });
    }

    const [updated] = invitation.status === "pending"
      ? await tx
          .update(organization_invitation)
          .set({ status: nextStatus })
          .where(and(where, eq(organization_invitation.status, "pending")))
          .returning()
      : [];

    const result = updated ?? (invitation.status === nextStatus
      ? invitation
      : (await tx
          .select()
          .from(organization_invitation)
          .where(where)
          .limit(1))[0]);

    if (!result || result.status !== nextStatus) {
      throw createError({ statusCode: 409, statusMessage: "Invitation has already been processed" });
    }

    if (action === "accept") {
      await tx
        .insert(organization_member)
        .values({
          id: uuidv7(),
          organization_id: organizationId,
          user_id: session.user.id,
          role: result.role,
        })
        .onConflictDoNothing({
          target: [organization_member.user_id, organization_member.organization_id],
        });
    }

    if (updated) {
      await logEvent(organizationId, eventName, {
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

    return result;
  });
});
