import { active_organization, organization, organization_member } from "~~/server/schema";
import { activeOrganizationScope, getIdentityDb, withTenantDb } from "~~/server/utils/db";
import { eq, and } from "drizzle-orm";
import z from "zod";

const activeOrganizationSchema = z.object({
  organization_id: z.string().uuid("Organization ID must be a valid UUID"),
});

export default defineEventHandler(async (event) => {
  const session = await requireSession(event);
  const body = activeOrganizationSchema.safeParse(await readBody(event));

  if (!body.success) {
    throw createError({
      statusCode: 400,
      statusMessage: body.error.issues[0]?.message || "A valid organization ID is required",
    });
  }
  const { organization_id: organizationId } = body.data;

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

  const organizationScope = await activeOrganizationScope(session.user.id, organizationId);
  const [updated, org] = await withTenantDb(organizationScope, async (tx) => {
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
