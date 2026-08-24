import type { api_key_scopes_type } from "~~/server/schema";
import { api_keys, api_key_scopes } from "~~/server/schema";
import { uuidv7 } from "uuidv7";
import { withTenantDb } from "~~/server/utils/db";
import { requireOwner } from "~~/server/utils/authorization";
import { API_KEY_SCOPE_VALUES } from "@cplane/migrations/utils";
import { generateSecret, hashApiKey } from "~~/server/utils/api-keys";

export default defineEventHandler(async (event) => {
  const membership = await requireOwner(event);

  const body = await readBody(event);
  const { name, scopes, expires_at, allowed_ips } = body as { name: string; scopes: Record<string, boolean>; expires_at?: number | null; allowed_ips?: string | null };

  if (!name || typeof name !== "string") {
    throw createError({
      statusCode: 400,
      statusMessage: "Name is required and must be a string",
    });
  }

  if (!scopes || typeof scopes !== "object" || Array.isArray(scopes)) {
    throw createError({
      statusCode: 400,
      statusMessage: "Scopes must be an object with boolean values",
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
    .map(([scope]) => scope);
  if (!enabledScopeNames.length || enabledScopeNames.some((scope) => !API_KEY_SCOPE_VALUES.includes(scope as typeof API_KEY_SCOPE_VALUES[number]))) {
    throw createError({
      statusCode: 400,
      statusMessage: "Select at least one valid scope",
    });
  }

  if (expires_at != null && (!Number.isInteger(expires_at) || expires_at < 1 || expires_at > 1200)) {
    throw createError({
      statusCode: 400,
      statusMessage: "Expiration must be a whole number of months between 1 and 1200",
    });
  }

  const organization_id = membership.organization_id;
  const api_key = await withTenantDb([organization_id], async (tx) => {
    const key = generateSecret();
    const keyHash = hashApiKey(key);
    const keyId = uuidv7();
    const now = new Date().toISOString();

    const [insertedKey] = await tx
      .insert(api_keys)
      .values({
        id: keyId,
        organization_id,
        name,
        key_hash: keyHash,
        created_at: now,
        expires_at: expires_at ?? null,
        allowed_ips: allowed_ips ?? null,
      })
      .returning();

    if (!insertedKey) {
      throw createError({
        statusCode: 500,
        statusMessage: "Failed to create API key",
      });
    }

    const enabledScopes = enabledScopeNames.map((scope) => ({
        id: uuidv7(),
        api_key_id: keyId,
        scope: scope as typeof api_key_scopes_type.enumValues[number],
        organization_id,
      }));

    if (enabledScopes.length > 0) {
      await tx.insert(api_key_scopes).values(enabledScopes);
    }

    await logEvent(organization_id, "api-key:created", {
      id: insertedKey.id,
      organization_id,
      name: insertedKey.name,
      created_at: insertedKey.created_at,
      expires_at: insertedKey.expires_at ?? null,
      scopes: enabledScopeNames,
    }, false, {}, tx);

    return {
      id: insertedKey.id,
      name: insertedKey.name,
      created_at: insertedKey.created_at,
      expires_at: insertedKey.expires_at ?? null,
      allowed_ips: insertedKey.allowed_ips ?? null,
      key, // Return the raw key only on creation
    };
  });

  return {
    ...api_key,
    scopes: enabledScopeNames,
  };
});
