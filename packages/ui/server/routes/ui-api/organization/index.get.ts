import { organization, organization_member } from "~~/server/schema";
import { getIdentityDb } from "~~/server/utils/db";
import { ilike, count, eq, and, or } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const query = getQuery(event);
  const search = query.search as string | undefined;
  const limit = Math.min(parseInt(query.limit as string) || 50, 100);
  const offset = parseInt(query.offset as string) || 0;

  const session = await requireSession(event);
  const identityDb = getIdentityDb();

  const organizationsQuery = identityDb
    .select({
      id: organization.id,
      name: organization.name,
      slug: organization.slug,
      created_at: organization.created_at,
      logo: organization.logo,
      member: {
        id: organization_member.user_id,
        role: organization_member.role,
      },
    })
    .from(organization)
    .innerJoin(organization_member, and(
      eq(organization_member.user_id, session.user.id),
      eq(organization_member.organization_id, organization.id)
    ))
    .limit(limit)
    .offset(offset)
    .$dynamic();

  const countQuery = identityDb
    .select({ count: count() })
    .from(organization)
    .innerJoin(organization_member, and(
      eq(organization_member.user_id, session.user.id),
      eq(organization_member.organization_id, organization.id)
    ))
    .$dynamic();
    
  if (search) {
    countQuery.where(
      or(ilike(organization.name, `%${search}%`), ilike(organization.slug, `%${search}%`))
    );
    organizationsQuery.where(
      or(ilike(organization.name, `%${search}%`), ilike(organization.slug, `%${search}%`))
    );
  }

  const organizations = await organizationsQuery;

  const totalResult = await countQuery;
  const total = totalResult[0]?.count || 0;

  return {
    data: organizations,
    pagination: {
      total,
      limit,
      offset,
    },
  };
});
