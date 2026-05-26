import { organization } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { withTenantDb } from "~~/server/utils/db";

export default defineEventHandler(async (event) => {
  const membership = await getOrganizationMembership(event);

  const organizationId = membership.organization_id;
  if (!organizationId) {
    throw createError({
      statusCode: 400,
      statusMessage: "Organization ID is required",
    });
  }

  const existingOrganization = await withTenantDb([organizationId], async (tx) => {
    const rows = await tx
      .select()
      .from(organization)
      .where(eq(organization.id, organizationId))
      .limit(1);

    if (!rows || rows.length === 0) {
      throw createError({
        statusCode: 404,
        statusMessage: "Organization not found",
      });
    }

    const org = rows[0]!;

    await tx.delete(organization).where(eq(organization.id, organizationId));

    return org;
  });

  if (existingOrganization.polar_customer_id) {
    try {
      await polar.customers.delete({ id: existingOrganization.polar_customer_id });
    } catch (error) {
      console.error(`Failed to delete Polar customer ${existingOrganization.polar_customer_id}:`, error);
    }
  }

  return { success: true };
});
