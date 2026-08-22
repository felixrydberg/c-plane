import { auth } from "./auth";
import { getIdentityDb } from "./db";
import { organization_member, user, organization_member_permission } from "~~/server/schema";
import { eq, and } from "drizzle-orm";
import type { H3Event } from "h3";
import { hasScope, type Subject } from "./permissions";

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

  const membership =
    userMembership.role === "owner"
      ? userMembership
      : { ...userMembership, role: "member" };

  return withPermissions(membership)
}

export async function withPermissions<M extends { id: string; role: string }>(
  membership: M,
): Promise<M & Subject> {
  const rows = await getIdentityDb()
    .select({ scope: organization_member_permission.scope })
    .from(organization_member_permission)
    .where(eq(organization_member_permission.member_id, membership.id));

  return { ...membership, permissions: rows.map((row) => row.scope) };
}

export async function permissionsForUser(
  organization_id: string,
  user_id: string,
): Promise<string[]> {
  const rows = await getIdentityDb()
    .select({ scope: organization_member_permission.scope })
    .from(organization_member_permission)
    .innerJoin(
      organization_member,
      eq(organization_member_permission.member_id, organization_member.id),
    )
    .where(
      and(
        eq(organization_member.organization_id, organization_id),
        eq(organization_member.user_id, user_id),
      ),
    );
  return rows.map((row) => row.scope);
}

export function assertAllowed(denial: string | null) {
  if (denial) {
    throw createError({
      statusCode: 403,
      statusMessage: denial,
    });
  }
}

export async function requireScope(event: H3Event, scope: string, organization_id?: string) {
  const membership = await getOrganizationMembership(event, organization_id);
  assertAllowed(hasScope(membership, scope) ? null : `Missing required permission: ${scope}`);
  return membership;
}

export async function requireOwner(event: H3Event, organization_id?: string) {
  const membership = await getOrganizationMembership(event, organization_id);
  if (membership.role !== "owner") {
    throw createError({
      statusCode: 403,
      statusMessage: "Only organization owners can perform this action",
    });
  }
  return membership;
}
