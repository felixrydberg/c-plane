import { requireSession } from "~~/server/utils/authorization";
import { db } from "~~/server/utils/auth";
import { cluster } from "~~/server/schema";
import {
  issueJoinCredential,
  MAX_JOIN_CREDENTIAL_TTL_MINUTES,
} from "~~/server/utils/cluster-join-credentials";
import { uuidv7 } from "uuidv7";
import { z } from "zod";

const createClusterSchema = z.object({
  region_id: z.string().uuid("region_id must be a valid UUID"),
  slug: z.string().trim().min(1, "slug is required"),
  name: z.string().trim().min(1, "name is required"),
  agent_id: z.string().trim().min(1, "agent_id is required"),
  agent_endpoint: z.string().trim().min(1, "agent_endpoint is required"),
  status: z.enum(["pending", "bootstrapping", "healthy", "draining", "offline", "removed"]).optional(),
  capacity_allocatable: z.number().int().nonnegative().optional(),
  capacity_used: z.number().int().nonnegative().optional(),
  health_status: z.enum(["healthy", "degraded", "offline"]).optional(),
  issue_join_credential: z.boolean().optional(),
  join_credential_ttl_minutes: z.number().int().positive().max(MAX_JOIN_CREDENTIAL_TTL_MINUTES).optional(),
});

export default defineEventHandler(async (event) => {
  const session = await requireSession(event);
  const parsed = createClusterSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: parsed.error.issues[0]?.message || "Invalid request body" });
  }
  const body = parsed.data;

  const result = await db.transaction(async (tx) => {
    const [created] = await tx.insert(cluster).values({
      id: uuidv7(),
      region_id: body.region_id,
      slug: body.slug,
      name: body.name,
      agent_id: body.agent_id,
      agent_endpoint: body.agent_endpoint,
      status: body.status ?? "pending",
      capacity_allocatable: body.capacity_allocatable ?? 0,
      capacity_used: body.capacity_used ?? 0,
      health_status: body.health_status ?? "healthy",
    }).returning();

    if (!created) {
      throw createError({ statusCode: 500, statusMessage: "Failed to create cluster" });
    }

    const joinCredential = await issueJoinCredential(tx, {
      clusterId: created.id,
      issuedByUserId: session.user.id,
      ttlMinutes: body.join_credential_ttl_minutes,
      revokeExisting: false,
    });

    return { cluster: created, join_credential: joinCredential };
  });

  event.res.status = 201;
  return result;
});
