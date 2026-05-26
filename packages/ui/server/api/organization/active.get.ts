
import { active_organization, organization, organization_member } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { getIdentityDb } from "~~/server/utils/db";

export default defineEventHandler(async (event) => {
  const session = await requireSession(event);

  const activeOrganization = await getIdentityDb().select({
    id: organization.id,
    name: organization.name,
    slug: organization.slug,
    created_at: organization.created_at,
    logo: organization.logo,
    member: {
      id: organization_member.user_id,
      role: organization_member.role,
    },
  }).from(active_organization)
    .innerJoin(organization_member, eq(active_organization.organization_id, organization_member.organization_id))
    .innerJoin(organization, eq(organization.id, active_organization.organization_id))
    .where(eq(organization_member.user_id, session.user.id))
    .limit(1);

  if (!activeOrganization || activeOrganization.length === 0) {
    throw createError({
      statusCode: 404,
      statusMessage: "No active organization found for the user",
    });
  }
  
  return activeOrganization[0];
});
