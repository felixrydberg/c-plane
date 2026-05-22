import { requireAdmin } from "~~/server/utils/authorization";
import { withAdminDb } from "~~/server/utils/db";
import { cluster } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { z } from "zod";

const listClustersQuerySchema = z.object({
  region_id: z.string().uuid("region_id must be a valid UUID").optional(),
});

export default defineEventHandler(async (event) => {
  await requireAdmin(event);
  const parsed = listClustersQuerySchema.safeParse(getQuery(event));
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: parsed.error.issues[0]?.message || "Invalid query params" });
  }

  const { region_id } = parsed.data;

  return withAdminDb((db) => {
    if (region_id) {
      return db.select().from(cluster).where(eq(cluster.region_id, region_id));
    }

    return db.select().from(cluster);
  });
});
