import { and, eq } from "drizzle-orm";
import { uuidv7 } from "uuidv7";
import {
  active_organization,
  organization,
  organization_invitation,
  organization_member,
} from "~~/server/schema";
import { activeOrganizationScope, getIdentityDb, withTenantDb } from "~~/server/utils/db";
import { logEvent } from "~~/server/utils/events";

export const acceptInvitationAndActivateOrganization = async (
  event: Parameters<typeof requireSession>[0],
  invitationId: string,
) => {
  const session = await requireSession(event);

  const [invitation] = await getIdentityDb()
    .select()
    .from(organization_invitation)
    .where(
      and(
        eq(organization_invitation.id, invitationId),
        eq(organization_invitation.email, session.user.email || ""),
      ),
    )
    .limit(1);

  if (!invitation) {
    throw createError({
      statusCode: 404,
      statusMessage: "Invitation not found or email does not match",
    });
  }

  if (invitation.status === "declined" || invitation.status === "revoked") {
    throw createError({
      statusCode: 400,
      statusMessage: "Invitation can no longer be accepted",
    });
  }

  if (new Date(invitation.expires_at) < new Date()) {
    throw createError({
      statusCode: 400,
      statusMessage: "Invitation has expired",
    });
  }

  const [organizationRecord] = await getIdentityDb()
    .select({
      id: organization.id,
      name: organization.name,
      slug: organization.slug,
    })
    .from(organization)
    .where(eq(organization.id, invitation.organization_id))
    .limit(1);

  if (!organizationRecord) {
    throw createError({
      statusCode: 404,
      statusMessage: "Organization not found",
    });
  }

  const organizationScope = await activeOrganizationScope(session.user.id, invitation.organization_id);

  if (invitation.status === "accepted") {
    await withTenantDb(organizationScope, async (tx) => {
      await tx
        .insert(active_organization)
        .values({
          user_id: session.user.id,
          organization_id: invitation.organization_id,
        })
        .onConflictDoUpdate({
          target: active_organization.user_id,
          set: {
            organization_id: invitation.organization_id,
          },
        });
    });

    return {
      invitation,
      organization: organizationRecord,
    };
  }

  const [updatedInvitation] = await withTenantDb(organizationScope, async (tx) => {
    const [updated] = await tx
      .update(organization_invitation)
      .set({ status: "accepted" })
      .where(eq(organization_invitation.id, invitation.id))
      .returning();

    if (!updated) {
      throw createError({
        statusCode: 500,
        statusMessage: "Failed to accept invitation"
      })
    }

    await tx
      .insert(organization_member)
      .values({
        id: uuidv7(),
        organization_id: invitation.organization_id,
        user_id: session.user.id,
        role: invitation.role,
      })
      .onConflictDoNothing({
        target: [organization_member.user_id, organization_member.organization_id],
      });

    await tx
      .insert(active_organization)
      .values({
        user_id: session.user.id,
        organization_id: invitation.organization_id,
      })
      .onConflictDoUpdate({
        target: active_organization.user_id,
        set: {
          organization_id: invitation.organization_id,
        },
      });

    await logEvent(invitation.organization_id, "organization:invitation_accepted", {
      id: updated.id,
      organization_id: updated.organization_id,
      email: updated.email,
      role: updated.role,
      status: updated.status,
      expires_at: updated.expires_at,
      inviter_id: updated.inviter_id,
      created_at: updated.created_at,
    }, false, {}, tx);

    return [updated];
  });

  if (!updatedInvitation) {
    throw createError({
      statusCode: 500,
      statusMessage: "Failed to accept invitation",
    });
  }

  return {
    invitation: updatedInvitation,
    organization: organizationRecord,
  };
};
