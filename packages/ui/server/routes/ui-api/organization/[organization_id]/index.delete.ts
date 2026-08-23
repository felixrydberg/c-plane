import { organization } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { withTenantDb } from "~~/server/utils/db";
import { requireOwner } from "~~/server/utils/authorization";

export default defineEventHandler(async (event) => {
  const membership = await requireOwner(event);

  const organizationId = membership.organization_id;
  if (!organizationId) {
    throw createError({
      statusCode: 400,
      statusMessage: "Organization ID is required",
    });
  }

  await withTenantDb([organizationId], async (tx) => {
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

    await tx.delete(organization).where(eq(organization.id, organizationId));
  });

  return { success: true };
});
