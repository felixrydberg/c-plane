import { organization, organization_member } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { eq, and } from "drizzle-orm";
import type { Project } from '@cplane/sdk';

export default defineEventHandler(async (event) => {
  const membership = await getOrganizationMembership(event);
  const organizationId = membership.organization_id;

  const _organization = await withTenantDb([organizationId], async (tx) => {
    const result = await tx.select({
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
      .limit(1);

    return result;
  });

  if (_organization.length === 0) {
    throw createError({
      statusCode: 404,
      statusMessage: "Organization not found",
    });
  }

  let projects: Project[] = [];
  try {
    const backendUrl = useRuntimeConfig().backendUrl;
    const headers = getRequestHeaders(event);
    const response = await $fetch(`${backendUrl}/api/organization/${organizationId}/projects`, {
      headers: headers as Record<string, string>,
    });
    projects = (response as any)?.data ?? [];
  } catch {
    projects = [];
  }

  return {
    ..._organization[0],
    projects,
  };
});
