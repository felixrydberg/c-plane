import { organization_member } from "~~/server/schema";
import { withTenantDb, getIdentityDb } from "~~/server/utils/db";
import { and, eq, count } from "drizzle-orm";
import { logEvent } from "~~/server/utils/events";
import {
  getOrganizationMembership,
  assertAllowed,
  withPermissions,
} from "~~/server/utils/authorization";
import { denyUpdateMemberRole } from "~~/server/utils/permissions";

export default defineEventHandler(async (event) => {
  const params = getRouterParams(event);
  const organization_id = params.organization_id as string;
  const member_id = params.member_id as string;

  if (!member_id) {
    throw createError({
      statusCode: 400,
      statusMessage: "Member ID is required",
    });
  }

  const actor = await getOrganizationMembership(event, organization_id);

  const body = await readBody<{ role?: unknown }>(event);

  const [target] = await getIdentityDb()
    .select({
      id: organization_member.id,
      role: organization_member.role,
      organization_id: organization_member.organization_id,
    })
    .from(organization_member)
    .where(eq(organization_member.id, member_id))
    .limit(1);

  if (!target || target.organization_id !== organization_id) {
    throw createError({
      statusCode: 404,
      statusMessage: "Member not found",
    });
  }

  return withTenantDb([organization_id], async (tx) => {
    const ownerRows = await tx
      .select({ value: count() })
      .from(organization_member)
      .where(and(
        eq(organization_member.organization_id, organization_id),
        eq(organization_member.role, "owner"),
      ));

    const isSelf = actor.id === target.id;
    const ownerCount = Number(ownerRows[0]?.value ?? 0);
    assertAllowed(denyUpdateMemberRole(actor, body?.role, target, {
      isSelf,
      ownerCount,
    }));

    const role = body.role as string;
    const [updated] = await tx
      .update(organization_member)
      .set({ role })
      .where(eq(organization_member.id, target.id))
      .returning();

    if (updated) {
      await logEvent(organization_id, "organization:member_updated", {
        id: updated.id,
        organization_id,
        user_id: updated.user_id,
        role: updated.role,
        previous_role: target.role,
      }, false, {}, tx);
    }

    return updated;
  }).then(async (updated) => {
    if (!updated) {
      throw createError({
        statusCode: 404,
        statusMessage: "Member not found",
      });
    }
    return withPermissions(updated);
  });
});
