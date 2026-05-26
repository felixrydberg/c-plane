import { active_organization, organization_member } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import { and, desc, eq, ne } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const session = await requireSession(event);
  const { organization_id } = getRouterParams(event);

  if (!organization_id) {
    throw createError({
      statusCode: 400,
      statusMessage: "Organization ID is required",
    });
  }

  const membership = await withTenantDb([organization_id], async (tx) =>
    tx.select().from(organization_member)
      .where(and(
        eq(organization_member.user_id, session.user.id),
        eq(organization_member.organization_id, organization_id),
      ))
      .limit(1),
  );

  if (!membership[0]) {
    throw createError({
      statusCode: 404,
      statusMessage: "Membership not found",
    });
  }

  if (membership[0].role === "owner" || membership[0].role === "admin") {
    throw createError({
      statusCode: 403,
      statusMessage: "Admins and owners cannot leave from this action",
    });
  }

  await withTenantDb([organization_id], async (tx) => {
    await tx.delete(organization_member)
      .where(eq(organization_member.id, membership[0].id));

    const activeOrganization = await tx.select().from(active_organization)
      .where(eq(active_organization.user_id, session.user.id))
      .limit(1);

    if (!activeOrganization[0] || activeOrganization[0].organization_id !== organization_id) {
      return;
    }

    const nextMembership = await tx.select({
      organization_id: organization_member.organization_id,
    })
      .from(organization_member)
      .where(and(
        eq(organization_member.user_id, session.user.id),
        ne(organization_member.organization_id, organization_id),
      ))
      .orderBy(desc(organization_member.created_at))
      .limit(1);

    if (!nextMembership[0]) {
      await tx.delete(active_organization)
        .where(eq(active_organization.user_id, session.user.id));
      return;
    }

    await tx.update(active_organization)
      .set({ organization_id: nextMembership[0].organization_id })
      .where(eq(active_organization.user_id, session.user.id));
  });
});
