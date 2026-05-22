import { eq } from "drizzle-orm";

import { withAdminDb } from "~~/server/utils/db";
import { requireAdmin } from "~~/server/utils/authorization";
import { cluster } from "~~/server/schema";
import { issueJoinCredential, issueJoinCredentialSchema } from "~~/server/utils/cluster-join-credentials";

export default defineEventHandler(async (event) => {
  const session = await requireAdmin(event);

  const clusterId = getRouterParam(event, "cluster_id");
  if (!clusterId) {
    throw createError({ statusCode: 400, statusMessage: "Missing cluster_id" });
  }

  const parsed = issueJoinCredentialSchema.safeParse((await readBody(event)) ?? {});
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: parsed.error.issues[0]?.message || "Invalid request body" });
  }

  const created = await withAdminDb(async (db) => {
    const foundCluster = await db.query.cluster.findFirst({
      where: eq(cluster.id, clusterId),
    });

    if (!foundCluster) {
      throw createError({ statusCode: 404, statusMessage: "Cluster not found" });
    }

    return issueJoinCredential(db, {
      clusterId,
      issuedByUserId: session.user.id,
      ttlMinutes: parsed.data.ttl_minutes,
      revokeExisting: parsed.data.revoke_existing,
    });
  });

  event.res.status = 201;
  return {
    ...created,
  };
});