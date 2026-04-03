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

  const found = await db.query.cluster.findFirst({
    where: eq(cluster.id, clusterId),
  });

  if (!found) {
    throw createError({ statusCode: 404, statusMessage: "Cluster not found" });
  }

  return found;
});
