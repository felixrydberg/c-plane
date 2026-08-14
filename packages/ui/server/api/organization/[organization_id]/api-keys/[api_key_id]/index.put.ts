import { withTenantDb } from "~~/server/utils/db";
import type { api_key_scopes_type } from "~~/server/schema";
import { api_keys, api_key_scopes } from "~~/server/schema";
import { eq, and, notInArray } from "drizzle-orm";
import { getOrganizationMembership } from "~~/server/utils/authorization";
import { uuidv7 } from "uuidv7";
import { API_KEY_SCOPE_VALUES } from "@cplane/migrations/utils";

export default defineEventHandler(async (event) => {
  const params = getRouterParams(event);
  const organization_id = params.organization_id as string;
  const api_key_id = params.api_key_id as string;

  await getOrganizationMembership(event, organization_id);

  const body = await readBody(event);
  const { name, scopes, allowed_ips } = body as { name: string; scopes: Record<string, boolean>; allowed_ips?: string | null };

  if (!scopes || typeof scopes !== "object" || Array.isArray(scopes)) {
    throw createError({
      statusCode: 400,
      statusMessage: "Scopes must be an object",
    });
  }

  if (Object.values(scopes).some((v) => typeof v !== "boolean")) {
    throw createError({
      statusCode: 400,
      statusMessage: "Scopes must be an object with boolean values",
    });
  }

  const enabledScopeNames = Object.entries(scopes)
    .filter(([, enabled]) => enabled)
    .map(([scope]) => scope as typeof api_key_scopes_type.enumValues[number]);
  if (!enabledScopeNames.length || enabledScopeNames.some((scope) => !API_KEY_SCOPE_VALUES.includes(scope))) {
    throw createError({
      statusCode: 400,
      statusMessage: "Select at least one valid scope",
    });
  }

  const updatedKey = await withTenantDb([organization_id], async (tx) => {
    const keyRecord = await tx
      .select()
      .from(api_keys)
      .where(
        and(
          eq(api_keys.id, api_key_id),
          eq(api_keys.organization_id, organization_id)
        )
      )
      .limit(1);

    if (!keyRecord || keyRecord.length === 0) {
      throw createError({
        statusCode: 404,
        statusMessage: "API key not found",
      });
    }

    if (name && typeof name === "string") {
      await tx
        .update(api_keys)
        .set({ name, allowed_ips: allowed_ips ?? null })
        .where(
          and(
            eq(api_keys.id, api_key_id),
            eq(api_keys.organization_id, organization_id)
          )
        );
    }

    const currentScopes = await tx
      .select({ scope: api_key_scopes.scope })
      .from(api_key_scopes)
      .where(eq(api_key_scopes.api_key_id, api_key_id));

    const currentScopeNames = currentScopes.map((s) => s.scope);
    const scopesToDelete = currentScopeNames.filter(
      (scope) => !enabledScopeNames.includes(scope as typeof api_key_scopes_type.enumValues[number])
    );

    if (scopesToDelete.length > 0) {
      await tx
        .delete(api_key_scopes)
        .where(
          and(
            eq(api_key_scopes.api_key_id, api_key_id),
            notInArray(api_key_scopes.scope, enabledScopeNames)
          )
        );
    }

    const scopesToInsert = enabledScopeNames.filter(
      (scope) => !currentScopeNames.includes(scope)
    );

    if (scopesToInsert.length > 0) {
      await tx.insert(api_key_scopes).values(
        scopesToInsert.map((scope) => ({
          id: uuidv7(),
          api_key_id,
          scope: scope as typeof api_key_scopes_type.enumValues[number],
          organization_id,
        }))
      );
    }

    const updated = await tx
      .select()
      .from(api_keys)
      .where(eq(api_keys.id, api_key_id))
      .limit(1);

    if (!updated[0]) {
      throw createError({
        statusCode: 404,
        statusMessage: "API key not found",
      });
    }

    await logEvent(organization_id, "api-key:updated", {
      id: updated[0].id,
      organization_id,
      name: updated[0].name,
      created_at: updated[0].created_at,
      scopes: Object.entries(scopes)
        .filter(([, enabled]) => enabled)
        .map(([scope]) => scope),
    }, false, {}, tx);

    return { scopesToDelete, scopesToInsert, updatedKeyData: updated[0] };
  });

  return {
    ...updatedKey.updatedKeyData,
    scopes: Object.entries(scopes)
      .filter(([, enabled]) => enabled)
      .map(([scope]) => scope),
  };
});
