import { requireSession } from "~~/server/utils/authorization";
import { db } from "~~/server/utils/auth";
import { cluster } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { z } from "zod";

const updateClusterSchema = z.object({
  region_id: z.uuid("region_id must be a valid UUID").optional(),
  slug: z.string().trim().min(1).optional(),
  name: z.string().trim().min(1).optional(),
  agent_id: z.string().trim().min(1).optional(),
  agent_endpoint: z.string().trim().min(1).optional(),
  status: z.enum(["pending", "bootstrapping", "healthy", "draining", "offline", "removed"]).optional(),
  capacity_allocatable: z.number().int().nonnegative().optional(),
  capacity_used: z.number().int().nonnegative().optional(),
  health_status: z.enum(["healthy", "degraded", "offline"]).optional(),
}).refine((value) => Object.keys(value).length > 0, {
  message: "At least one field is required",
});

export default defineEventHandler(async (event) => {
  await requireSession(event);
  const clusterId = getRouterParam(event, "cluster_id");
  if (!clusterId) {
    throw createError({ statusCode: 400, statusMessage: "Missing cluster_id" });
  }

  const parsed = updateClusterSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: parsed.error.issues[0]?.message || "Invalid request body" });
  }
  const body = parsed.data;

  const [updated] = await db
    .update(cluster)
    .set({
      region_id: body.region_id,
      slug: body.slug,
      name: body.name,
      agent_id: body.agent_id,
      agent_endpoint: body.agent_endpoint,
      status: body.status,
      capacity_allocatable: body.capacity_allocatable,
      capacity_used: body.capacity_used,
      health_status: body.health_status,
      updated_at: new Date().toISOString(),
    })
    .where(eq(cluster.id, clusterId))
    .returning();

  if (!updated) {
    throw createError({ statusCode: 404, statusMessage: "Cluster not found" });
  }

  return updated;
});
