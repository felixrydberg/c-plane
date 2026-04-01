import { active_organization, organization, organization_member } from "~~/server/schema";
import { db } from "~~/server/utils/auth";
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

  const membership = await db.select().from(organization_member)
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

  const updated = await db
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

  const org = await db.select({
    id: organization.id,
    slug: organization.slug,
  }).from(organization)
    .where(eq(organization.id, organizationId))
    .limit(1);

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
