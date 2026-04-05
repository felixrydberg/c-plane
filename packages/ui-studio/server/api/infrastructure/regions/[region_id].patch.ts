import { requireSession } from "~~/server/utils/authorization";
import { db } from "~~/server/utils/auth";
import { region } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { z } from "zod";

const updateRegionSchema = z.object({
  slug: z.string().trim().min(1).optional(),
  display_name: z.string().trim().min(1).optional(),
  s3_provider_id: z.uuid("s3_provider_id must be a valid UUID").nullable().optional(),
  status: z.enum(["active", "inactive", "maintenance"]).optional(),
}).refine((value) => Object.keys(value).length > 0, {
  message: "At least one field is required",
});

export default defineEventHandler(async (event) => {
  await requireSession(event);
  const regionId = getRouterParam(event, "region_id");
  if (!regionId) {
    throw createError({ statusCode: 400, statusMessage: "Missing region_id" });
  }

  const parsed = updateRegionSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: parsed.error.issues[0]?.message || "Invalid request body" });
  }
  const body = parsed.data;

  const [updated] = await db
    .update(region)
    .set({
      slug: body.slug,
      display_name: body.display_name,
      s3_provider_id: body.s3_provider_id,
      status: body.status,
      updated_at: new Date().toISOString(),
    })
    .where(eq(region.id, regionId))
    .returning();

  if (!updated) {
    throw createError({ statusCode: 404, statusMessage: "Region not found" });
  }

  return updated;
});
