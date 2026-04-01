import { db } from "~~/server/utils/auth";
import { organization, organization_member } from "~~/server/schema";
import { eq, and } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const membership = await getOrganizationMembership(event);

  const body = await readBody(event);
  const name = body.name as string | undefined;

  if (!name || name.trim() === "") {
    throw createError({
      statusCode: 400,
      statusMessage: "Organization name is required",
    });
  }

  await db
    .update(organization)
    .set({ name: name.trim() })
    .where(eq(organization.id, membership.organization_id));

  const _organization = await db.select({
    id: organization.id,
    name: organization.name,
    slug: organization.slug,
    created_at: organization.created_at,
    logo: organization.logo,
    member: {
      id: organization_member.user_id,
      role: organization_member.role,
    },
  }).from(organization)
    .where(eq(organization.id, membership.organization_id))
    .innerJoin(
      organization_member,
      and(
        eq(organization_member.organization_id, organization.id),
        eq(organization_member.user_id, membership.user_id)
      )
    )
    .limit(1)

  if (_organization.length === 0) {
    throw createError({
      statusCode: 404,
      statusMessage: "Organization not found",
    });
  }

  return _organization[0];
});
