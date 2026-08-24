import { getIdentityDb } from "~~/server/utils/db";
import { organization_invitation, organization, user } from "~~/server/schema";
import { eq, and, or, ilike, count } from "drizzle-orm";
import { pagination } from "~~/server/utils/pagination";

export default defineEventHandler(async (event) => {
  const session = await requireSession(event);

  const query = getQuery(event);
  const search = query.search as string | undefined;
  const { limit, offset } = pagination(query);
  const statusParam = query.status as string | undefined;

  const validStatuses = ['pending', 'accepted', 'declined', 'revoked'] as const;
  const status = statusParam && validStatuses.includes(statusParam as 'pending' | 'accepted' | 'declined' | 'revoked') 
    ? (statusParam as 'pending' | 'accepted' | 'declined' | 'revoked')
    : undefined;

  const identityDb = getIdentityDb();

  const where = and(
    eq(organization_invitation.email, session.user.email),
    status ? eq(organization_invitation.status, status) : undefined,
    search
      ? or(
          ilike(organization_invitation.email, `%${search}%`),
          ilike(user.name, `%${search}%`),
        )
      : undefined,
  );

  const invitationsQuery = identityDb
    .select({
      id: organization_invitation.id,
      email: organization_invitation.email,
      role: organization_invitation.role,
      status: organization_invitation.status,
      expires_at: organization_invitation.expires_at,
      organization_id: organization_invitation.organization_id,
      organization: {
        id: organization.id,
        name: organization.name,
        slug: organization.slug,
      },
      inviter: {
        id: user.id,
        name: user.name,
        email: user.email,
        image: user.image,
      },
    })
    .from(organization_invitation)
    .innerJoin(user, eq(organization_invitation.inviter_id, user.id))
    .innerJoin(organization, eq(organization_invitation.organization_id, organization.id))
    .where(where)
    .$dynamic();

  const invitations = await invitationsQuery.limit(limit).offset(offset);
  const countQuery = identityDb
    .select({ count: count() })
    .from(organization_invitation)
    .innerJoin(user, eq(organization_invitation.inviter_id, user.id))
    .where(where)
    .$dynamic();

  const totalResult = await countQuery;
  const total = totalResult[0]?.count || 0;

  return {
    data: invitations,
    pagination: {
      total,
      limit,
      offset,
      hasMore: total > offset + limit,
    },
  };
});
