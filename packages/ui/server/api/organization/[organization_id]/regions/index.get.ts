import { region } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { and, asc, eq, ne } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const membership = await getOrganizationMembership(event);
  const organizationId = membership?.organization_id;

  if (!organizationId) {
    throw createError({
      statusCode: 403,
      statusMessage: "Not a member of this organization",
    });
  }

  return withTenantDb([organizationId], (db) => {
    return db
      .select({
        id: region.id,
        slug: region.slug,
        display_name: region.display_name,
        status: region.status,
        routing_mode: region.routing_mode,
      })
      .from(region)
      .where(
        and(
          eq(region.status, "active"),
          ne(region.routing_mode, "disabled"),
        ),
      )
      .orderBy(asc(region.display_name));
  });
});
