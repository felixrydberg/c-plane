import { requireAdmin } from "~~/server/utils/authorization";
import { withAdminDb } from "~~/server/utils/db";
import { cluster, cluster_ingress_endpoint } from "~~/server/schema";
import { issueJoinCredential } from "~~/server/utils/cluster-join-credentials";
import { uuidv7 } from "uuidv7";
import { z } from "zod";

const ingressEndpointSchema = z.object({
  address: z.string().trim().min(1, "ingress endpoint address is required"),
  port: z.number().int().positive().max(65535).optional(),
  enabled: z.boolean().optional(),
});

const createClusterSchema = z.object({
  region_id: z.string().uuid("region_id must be a valid UUID"),
  slug: z.string().trim().min(1, "slug is required"),
  name: z.string().trim().min(1, "name is required"),
  ingress_endpoints: z.array(ingressEndpointSchema).optional(),
}).superRefine((value, ctx) => {
  if (!value.ingress_endpoints || value.ingress_endpoints.length <= 1) {
    return;
  }

  const seen = new Set<string>();
  for (const endpoint of value.ingress_endpoints) {
    const key = `${endpoint.address.toLowerCase()}:${endpoint.port ?? 443}`;
    if (seen.has(key)) {
      ctx.addIssue({
        code: "custom",
        path: ["ingress_endpoints"],
        message: "ingress_endpoints contains duplicate address/port entries",
      });
      return;
    }

    seen.add(key);
  }
});

export default defineEventHandler(async (event) => {
  const session = await requireAdmin(event);
  const parsed = createClusterSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: parsed.error.issues[0]?.message || "Invalid request body" });
  }
  const body = parsed.data;

  const result = await withAdminDb(async (db) => {
    const [created] = await db.insert(cluster).values({
      id: uuidv7(),
      region_id: body.region_id,
      slug: body.slug,
      name: body.name,
      health_status: "offline",
    }).returning();

    if (!created) {
      throw createError({ statusCode: 500, statusMessage: "Failed to create cluster" });
    }

    const joinCredential = await issueJoinCredential(db, {
      clusterId: created.id,
      issuedByUserId: session.user.id,
      revokeExisting: false,
    });

    const createdIngressEndpoints = body.ingress_endpoints?.length
      ? await db.insert(cluster_ingress_endpoint).values(
          body.ingress_endpoints.map((endpoint) => ({
            id: uuidv7(),
            cluster_id: created.id,
            address: endpoint.address,
            port: endpoint.port ?? 443,
            enabled: endpoint.enabled ?? true,
            health_status: "degraded" as const,
          })),
        ).returning()
      : [];

    return {
      cluster: created,
      join_credential: joinCredential,
      ingress_endpoints: createdIngressEndpoints,
    };
  });

  event.res.status = 201;
  return result;
});
