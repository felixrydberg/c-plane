import { organization_member, user, organization_member_permission } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { eq, and, or, ilike, count, ne, inArray } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const query = getQuery(event);
  const search = query.search as string | undefined;
  const limit = Math.min(parseInt(query.limit as string) || 50, 100);
  const offset = parseInt(query.offset as string) || 0;
  const excludeRequester = query.excludeRequester === "true" || query.excludeRequester === true;
  const excludeIds = typeof query.excludeIds === 'string' 
    ? query.excludeIds.split(',').filter(Boolean)
    : Array.isArray(query.excludeIds)
      ? query.excludeIds
      : [];

  const membership = await getOrganizationMembership(event);
  const { id: requesterId } = membership;
  const searchCondition = search
    ? or(
        ilike(user.name, `%${search}%`),
        ilike(user.email, `%${search}%`)
      )
    : undefined;

  return withTenantDb([membership.organization_id], async (tx) => {
    const membersQuery = tx
      .select({
        id: organization_member.id,
        role: organization_member.role,
        created_at: organization_member.created_at,
        organization_id: organization_member.organization_id,
        user: {
          id: user.id,
          name: user.name,
          email: user.email,
          image: user.image,
          emailVerified: user.emailVerified,
        },
      })
      .from(organization_member)
      .innerJoin(user, eq(organization_member.user_id, user.id))
      .$dynamic()
    
    const countQuery = tx
      .select({ count: count() })
      .from(organization_member)
      .innerJoin(user, eq(organization_member.user_id, user.id))
      .$dynamic();

    const conditions = [eq(organization_member.organization_id, membership.organization_id)];

    if (excludeRequester) {
      conditions.push(ne(organization_member.id, requesterId));
    }

    if (search && searchCondition) {
      conditions.push(searchCondition);
    }

    if (excludeIds.length > 0) {
      conditions.push(ne(organization_member.id, excludeIds[0]));
      for (let i = 1; i < excludeIds.length; i++) {
        conditions.push(ne(organization_member.id, excludeIds[i]));
      }
    }

    const finalCondition = and(...conditions);
    membersQuery.where(finalCondition);
    countQuery.where(finalCondition);

    const members = await membersQuery.limit(limit).offset(offset);

    const totalResult = await countQuery;
    const total = totalResult[0]?.count || 0;

    const memberIds = members.map((m) => m.id);
    const permissionRows = memberIds.length
      ? await tx
          .select({
            member_id: organization_member_permission.member_id,
            scope: organization_member_permission.scope,
          })
          .from(organization_member_permission)
          .where(inArray(organization_member_permission.member_id, memberIds))
      : [];
    const permissionsByMember = new Map<string, string[]>();
    for (const row of permissionRows) {
      const list = permissionsByMember.get(row.member_id) ?? [];
      list.push(row.scope);
      permissionsByMember.set(row.member_id, list);
    }

    return {
      data: members.map((m) => ({
        ...m,
        role: m.role === "owner" ? "owner" : "member",
        permissions: permissionsByMember.get(m.id) ?? [],
      })),
      pagination: {
        total,
        limit,
        offset,
      },
    };
  });
});
