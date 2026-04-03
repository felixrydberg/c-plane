import { requireSession } from "~~/server/utils/authorization";
import { db } from "~~/server/utils/auth";
import { cluster } from "~~/server/schema";
import { eq } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  await requireSession(event);
  const clusterId = getRouterParam(event, "cluster_id");
  if (!clusterId) {
    throw createError({ statusCode: 400, statusMessage: "Missing cluster_id" });
  }

  const deleted = await db
    .delete(cluster)
    .where(eq(cluster.id, clusterId))
    .returning({ id: cluster.id });

  if (deleted.length === 0) {
    throw createError({ statusCode: 404, statusMessage: "Cluster not found" });
  }

  event.res.status = 204;
  return null;
});
