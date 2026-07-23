import { withTenantDb } from "~~/server/utils/db";
import { organization, organization_member } from "~~/server/schema";
import { eq, and } from "drizzle-orm";
import { logEvent } from "~~/server/utils/events";
import z from "zod";

const renameOrganizationSchema = z.object({
  name: z.string().trim().min(1, "Organization name is required"),
});

export default defineEventHandler(async (event) => {
  const membership = await getOrganizationMembership(event);

  const body = await readBody(event);
  const parsed = renameOrganizationSchema.safeParse(body);
  if (!parsed.success) {
    throw createError({
      statusCode: 400,
      statusMessage: parsed.error.issues[0]?.message || "Invalid request body",
    });
  }
  const { name } = parsed.data;

  const organizationId = membership.organization_id;

  const _organization = await withTenantDb([organizationId], async (tx) => {
    await tx
      .update(organization)
      .set({ name })
      .where(eq(organization.id, membership.organization_id));

    await logEvent(
      organizationId,
      "organization:updated",
      {
        summary: `Updated organization name to '${name}'`,
        target_id: organizationId,
      },
      false,
      { actor_id: membership.user_id },
      tx,
    );

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

  return _organization[0];
});
