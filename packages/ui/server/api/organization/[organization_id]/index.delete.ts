import { organization } from "~~/server/schema";
import { eq } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const membership = await getOrganizationMembership(event);

  const organizationId = membership.organization_id;
  if (!organizationId) {
    throw createError({
      statusCode: 400,
      statusMessage: "Organization ID is required",
    });
  }

  const existingOrganization = await db
    .select()
    .from(organization)
    .where(eq(organization.id, organizationId))
    .limit(1);

  if (!existingOrganization || existingOrganization.length === 0) {
    throw createError({
      statusCode: 404,
      statusMessage: "Organization not found",
    });
  }

  const org = existingOrganization[0]!;

  await db.transaction(async (tx) => {
    await tx.delete(organization).where(eq(organization.id, organizationId));
  });

  if (org.polar_customer_id) {
    try {
      await polar.customers.delete({ id: org.polar_customer_id });
    } catch (error) {
      console.error(`Failed to delete Polar customer ${org.polar_customer_id}:`, error);
    }
  }

  return { success: true };
});
