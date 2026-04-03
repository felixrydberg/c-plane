import { requireSession } from "~~/server/utils/authorization";
import { db } from "~~/server/utils/auth";
import { cluster } from "~~/server/schema";
import { uuidv7 } from "uuidv7";
import { z } from "zod";

const createClusterSchema = z.object({
  region_id: z.string().uuid("region_id must be a valid UUID"),
  slug: z.string().trim().min(1, "slug is required"),
  name: z.string().trim().min(1, "name is required"),
  agent_id: z.string().trim().min(1, "agent_id is required"),
  agent_endpoint: z.string().trim().min(1, "agent_endpoint is required"),
  status: z.enum(["active", "inactive", "draining", "offline"]).optional(),
  capacity_allocatable: z.number().int().nonnegative().optional(),
  capacity_used: z.number().int().nonnegative().optional(),
  health_status: z.enum(["healthy", "degraded", "offline"]).optional(),
});

export default defineEventHandler(async (event) => {
  await requireSession(event);
  const parsed = createClusterSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: parsed.error.issues[0]?.message || "Invalid request body" });
  }
  const body = parsed.data;

  const [created] = await db.insert(cluster).values({
    id: uuidv7(),
    region_id: body.region_id,
    slug: body.slug,
    name: body.name,
    agent_id: body.agent_id,
    agent_endpoint: body.agent_endpoint,
    status: body.status ?? "active",
    capacity_allocatable: body.capacity_allocatable ?? 0,
    capacity_used: body.capacity_used ?? 0,
    health_status: body.health_status ?? "healthy",
  }).returning();

  event.res.status = 201;
  return created;
});
