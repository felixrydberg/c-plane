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

  const found = await withAdminDb((db) => db.query.cluster.findFirst({
    where: eq(cluster.id, clusterId),
  }));

  if (!found) {
    throw createError({ statusCode: 404, statusMessage: "Cluster not found" });
  }

  return found;
});
