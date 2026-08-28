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

  const acceptedInvitation = await withTenantDb(organizationScope, async (tx) => {
    const [currentInvitation] = await tx
      .select()
      .from(organization_invitation)
      .where(
        and(
          eq(organization_invitation.id, invitation.id),
          eq(organization_invitation.email, session.user.email || ""),
        ),
      )
      .limit(1);

    if (!currentInvitation) {
      throw createError({
        statusCode: 404,
        statusMessage: "Invitation not found or email does not match",
      });
    }

    if (currentInvitation.status === "declined" || currentInvitation.status === "revoked") {
      throw createError({
        statusCode: 400,
        statusMessage: "Invitation can no longer be accepted",
      });
    }

    if (new Date(currentInvitation.expires_at) < new Date()) {
      throw createError({
        statusCode: 400,
        statusMessage: "Invitation has expired",
      });
    }

    let accepted = currentInvitation;
    let newlyAccepted = false;

    if (currentInvitation.status === "pending") {
      const [updated] = await tx
        .update(organization_invitation)
        .set({ status: "accepted" })
        .where(
          and(
            eq(organization_invitation.id, currentInvitation.id),
            eq(organization_invitation.status, "pending"),
          ),
        )
        .returning();

      if (updated) {
        accepted = updated;
        newlyAccepted = true;
      } else {
        const [acceptedAfterRace] = await tx
          .select()
          .from(organization_invitation)
          .where(eq(organization_invitation.id, currentInvitation.id))
          .limit(1);

        if (!acceptedAfterRace || acceptedAfterRace.status !== "accepted") {
          throw createError({
            statusCode: 409,
            statusMessage: "Invitation has already been processed",
          });
        }

        accepted = acceptedAfterRace;
      }
    }

    await tx
      .insert(organization_member)
      .values({
        id: uuidv7(),
        organization_id: accepted.organization_id,
        user_id: session.user.id,
        role: accepted.role,
      })
      .onConflictDoNothing({
        target: [organization_member.user_id, organization_member.organization_id],
      });

    await tx
      .insert(active_organization)
      .values({
        user_id: session.user.id,
        organization_id: accepted.organization_id,
      })
      .onConflictDoUpdate({
        target: active_organization.user_id,
        set: {
          organization_id: accepted.organization_id,
        },
      });

    if (newlyAccepted) {
      await logEvent(accepted.organization_id, "organization:invitation_accepted", {
        id: accepted.id,
        organization_id: accepted.organization_id,
        email: accepted.email,
        role: accepted.role,
        status: accepted.status,
        expires_at: accepted.expires_at,
        inviter_id: accepted.inviter_id,
        created_at: accepted.created_at,
      }, false, {}, tx);
    }

    return accepted;
  });

  return {
    invitation: acceptedInvitation,
    organization: organizationRecord,
  };
};
