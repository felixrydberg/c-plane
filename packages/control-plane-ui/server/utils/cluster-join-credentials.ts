import { and, eq, isNull, sql } from "drizzle-orm";
import { createHash, randomBytes } from "node:crypto";
import { uuidv7 } from "uuidv7";
import { z } from "zod";

import { cluster_join_credential } from "~~/server/schema";
import type { db } from "~~/server/utils/auth";

export const DEFAULT_JOIN_CREDENTIAL_TTL_MINUTES = 15;
export const MAX_JOIN_CREDENTIAL_TTL_MINUTES = 24 * 60;

export const issueJoinCredentialSchema = z.object({
  ttl_minutes: z.number().int().positive().max(MAX_JOIN_CREDENTIAL_TTL_MINUTES).optional(),
  revoke_existing: z.boolean().optional(),
});

const hashJoinToken = (token: string) => {
  const pepper = process.env.CLUSTER_JOIN_TOKEN_PEPPER ?? "";
  return createHash("sha256").update(`${token}.${pepper}`).digest("hex");
};

const createJoinToken = () => `cj_${randomBytes(24).toString("hex")}`;

type DbClient = Pick<typeof db, "insert" | "update">;

export const issueJoinCredential = async (
  client: DbClient,
  params: {
    clusterId: string;
    issuedByUserId: string;
    ttlMinutes?: number;
    revokeExisting?: boolean;
  },
) => {
  const now = new Date().toISOString();
  const ttlMinutes = params.ttlMinutes ?? DEFAULT_JOIN_CREDENTIAL_TTL_MINUTES;
  const expiresAt = new Date(Date.now() + ttlMinutes * 60 * 1000).toISOString();

  if (params.revokeExisting ?? true) {
    await client.update(cluster_join_credential)
      .set({
        revoked_at: now,
        revoked_reason: "reissued",
        updated_at: now,
      })
      .where(and(
        eq(cluster_join_credential.cluster_id, params.clusterId),
        isNull(cluster_join_credential.used_at),
        isNull(cluster_join_credential.revoked_at),
        sql`${cluster_join_credential.expires_at} > now()`,
      ));
  }

  const token = createJoinToken();
  const [created] = await client.insert(cluster_join_credential).values({
    id: uuidv7(),
    cluster_id: params.clusterId,
    token_hash: hashJoinToken(token),
    expires_at: expiresAt,
    issued_by_user_id: params.issuedByUserId,
  }).returning();

  if (!created) {
    throw createError({ statusCode: 500, statusMessage: "Failed to issue join credential" });
  }

  return {
    id: created.id,
    cluster_id: created.cluster_id,
    token,
    expires_at: created.expires_at,
    created_at: created.created_at,
  };
};