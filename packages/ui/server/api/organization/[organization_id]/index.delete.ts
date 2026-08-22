import { organization } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { withTenantDb } from "~~/server/utils/db";
import { requireOwner, assertAllowed } from "~~/server/utils/authorization";
import { denyDeleteOrganization } from "~~/server/utils/permissions";

export default defineEventHandler(async (event) => {
  const membership = await requireOwner(event);

  const body = await readBody<{ confirm?: boolean }>(event).catch(() => ({ confirm: false }));
  if (body.confirm !== true) {
    throw createError({
      statusCode: 400,
      statusMessage: "Confirmation required: pass { \"confirm\": true } to delete this organization",
    });
  }

  assertAllowed(denyDeleteOrganization(membership));

  const organizationId = membership.organization_id;

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
