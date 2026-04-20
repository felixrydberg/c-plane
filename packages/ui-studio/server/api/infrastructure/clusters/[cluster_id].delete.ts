import { requireAdmin } from "~~/server/utils/authorization";
import { withAdminDb } from "~~/server/utils/db";
import { cluster } from "~~/server/schema";
import { eq } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  await requireAdmin(event);
  const clusterId = getRouterParam(event, "cluster_id");
  if (!clusterId) {
    throw createError({ statusCode: 400, statusMessage: "Missing cluster_id" });
  }

  const deleted = await withAdminDb((db) => {
    return db
      .delete(cluster)
      .where(eq(cluster.id, clusterId))
      .returning({ id: cluster.id });
  });

  if (deleted.length === 0) {
    throw createError({ statusCode: 404, statusMessage: "Cluster not found" });
  }

  event.res.status = 204;
  return null;
});
