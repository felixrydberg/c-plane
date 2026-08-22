import { organization_member, organization_member_permission } from "~~/server/schema";
import { withTenantDb, getIdentityDb } from "~~/server/utils/db";
import { eq } from "drizzle-orm";
import { uuidv7 } from "uuidv7";
import { logEvent } from "~~/server/utils/events";
import {
  getOrganizationMembership,
  assertAllowed,
  withPermissions,
} from "~~/server/utils/authorization";
import { denyAssignPermissions } from "~~/server/utils/permissions";
import { isMemberPermissionScope } from "@cplane/migrations/utils";

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

  const body = await readBody<{ permissions?: unknown }>(event);
  const requested = body?.permissions;

  if (!Array.isArray(requested) || requested.some((s) => typeof s !== "string")) {
    throw createError({
      statusCode: 400,
      statusMessage: "Permissions must be an array of scope strings",
    });
  }

  const permissions = [...new Set(requested as string[])];
  const invalid = permissions.find((scope) => !isMemberPermissionScope(scope));
  if (invalid) {
    throw createError({
      statusCode: 400,
      statusMessage: `Unknown permission: ${invalid}`,
    });
  }

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

  assertAllowed(denyAssignPermissions(actor, target));

  await withTenantDb([organization_id], async (tx) => {
    await tx
      .delete(organization_member_permission)
      .where(eq(organization_member_permission.member_id, member_id));

    if (permissions.length > 0) {
      await tx.insert(organization_member_permission).values(
        permissions.map((scope) => ({
          id: uuidv7(),
          member_id,
          organization_id,
          scope: scope as (typeof organization_member_permission.$inferInsert)["scope"],
        })),
      );
    }

    await logEvent(organization_id, "organization:permissions_updated", {
      id: target.id,
      organization_id,
      user_id: target.id,
      permissions,
    }, false, {}, tx);
  });

  return withPermissions({ id: member_id, role: target.role, permissions });
});
