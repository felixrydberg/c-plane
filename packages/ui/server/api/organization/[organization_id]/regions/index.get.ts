export default defineEventHandler(async (event) => {
  const membership = await getOrganizationMembership(event);
  const organizationId = membership?.organization_id;

  if (!organizationId) {
    throw createError({
      statusCode: 403,
      statusMessage: "Not a member of this organization",
    });
  }

  const config = useRuntimeConfig(event);
  return $fetch(`${config.controlPlaneUrl}/internal/regions`, {
    headers: { "x-cplane-token": config.cplaneServiceToken },
  });
});
