import { requireSession } from "~~/server/utils/authorization";
import { db } from "~~/server/utils/auth";
import { cluster } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { z } from "zod";

const listClustersQuerySchema = z.object({
  region_id: z.string().uuid("region_id must be a valid UUID").optional(),
});

export default defineEventHandler(async (event) => {
  await requireSession(event);
  const parsed = listClustersQuerySchema.safeParse(getQuery(event));
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: parsed.error.issues[0]?.message || "Invalid query params" });
  }

  const { region_id } = parsed.data;

  if (region_id) {
    return db.select().from(cluster).where(eq(cluster.region_id, region_id));
  }

  return db.select().from(cluster);
});
