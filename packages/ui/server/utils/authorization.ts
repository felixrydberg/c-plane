import { auth } from "./auth";
import { getIdentityDb } from "./db";
import { organization_member, user  } from "~~/server/schema";
import { eq, and } from "drizzle-orm";
import type { H3Event } from "h3";

export const requireSession = async (event: H3Event) => {
  const session = await auth.api.getSession({
    headers: event.headers,
  });

  if (!session) {
    throw createError({
      statusCode: 401,
      statusMessage: "Unauthorized",
    });
  }
  return session;
};

export async function getOrganizationMembership(event: H3Event, organization_id?: string) {
  const session = await requireSession(event);
  const query = getQuery(event);
  const params = getRouterParams(event);
  if (!organization_id) {
    organization_id = query.organization_id as string | undefined;
  }

  if (!organization_id && params && 'organization_id' in params) {
    organization_id = params.organization_id as string | undefined;
  }

  if (!organization_id && event.method !== "GET") {
    const body = await readBody(event);
    organization_id = body.organization_id as string | undefined;
  }

  if (!organization_id) {
    throw createError({
      statusCode: 400,
      statusMessage: "No organization specified",
    });
  }

  const [userMembership] = await getIdentityDb()
    .select({
      id: organization_member.id,
      organization_id: organization_member.organization_id,
      user_id: organization_member.user_id,
      role: organization_member.role,
      created_at: organization_member.created_at,
      user: {
        email: user.email,
        name: user.name,
        image: user.image,
      }
    })
    .from(organization_member)
    .innerJoin(user, eq(organization_member.user_id, user.id))
    .where(
      and(
        eq(organization_member.organization_id, organization_id),
        eq(organization_member.user_id, session.user.id)
      )
    )
    .limit(1);

  if (!userMembership) {
    throw createError({
      statusCode: 403,
      statusMessage: "Not a member of this organization",
    });
  }

  return userMembership
}

export async function requireOwner(event: H3Event, organization_id?: string) {
  const membership = await getOrganizationMembership(event, organization_id);
  if (membership.role !== "owner") {
    throw createError({
      statusCode: 403,
      statusMessage: "Organization owner role required",
    });
  }
  return membership;
}
