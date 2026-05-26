import { active_organization, organization, organization_member } from "~~/server/schema";
import { getIdentityDb, withTenantDb } from "~~/server/utils/db";
import { eq, and } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const session = await requireSession(event);
  const body = await readBody<{ organization_id?: string }>(event);

  const organizationId = body.organization_id;
  if (!organizationId) {
    throw createError({
      statusCode: 400,
      statusMessage: "Organization ID is required",
    });
  }

  const membership = await getIdentityDb().select().from(organization_member)
    .where(
      and(
        eq(organization_member.user_id, session.user.id),
        eq(organization_member.organization_id, organizationId),
      ),
    )
    .limit(1);

  if (!membership || membership.length === 0) {
    throw createError({
      statusCode: 403,
      statusMessage: "You do not have access to this organization",
    });
  }

  const [updated, org] = await withTenantDb([organizationId], async (tx) => {
    const upserted = await tx
      .insert(active_organization)
      .values({
        user_id: session.user.id,
        organization_id: organizationId,
      })
      .onConflictDoUpdate({
        target: active_organization.user_id,
        set: {
          organization_id: organizationId,
        },
      })
      .returning();

    const orgResult = await tx.select({
      id: organization.id,
      slug: organization.slug,
    }).from(organization)
      .where(eq(organization.id, organizationId))
      .limit(1);

    return [upserted, orgResult];
  });

  if (!org || org.length === 0) {
    throw createError({
      statusCode: 500,
      statusMessage: "Organization not found",
    });
  }

  return {
    ...updated[0],
    slug: org[0].slug,
  };
});
