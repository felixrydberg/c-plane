import { acceptInvitationAndActivateOrganization } from "~~/server/utils/invitations";

export default defineEventHandler(async (event) => {
  const params = getRouterParams(event);
  const invitationId = params.invitation_id as string;

  if (!invitationId) {
    throw createError({
      statusCode: 400,
      statusMessage: "Invitation ID is required",
    });
  }

  return await acceptInvitationAndActivateOrganization(event, invitationId);
});
