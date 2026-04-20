import { requireAdmin } from "~~/server/utils/authorization";
import { withAdminDb } from "~~/server/utils/db";
import { region } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { uuidv7 } from "uuidv7";
import { z } from "zod";
import { isReservedRegionSlug, normalizeRegionSlug, REGION_SLUG_REGEX } from "~~/server/utils/regions";

const createRegionSchema = z.object({
  slug: z
    .string()
    .trim()
    .min(1, "slug is required")
    .transform((value) => normalizeRegionSlug(value))
    .refine((value) => REGION_SLUG_REGEX.test(value), "slug is invalid")
    .refine((value) => !isReservedRegionSlug(value), "slug is reserved"),
  display_name: z.string().trim().min(1, "display_name is required"),
  s3_provider_id: z.uuid("s3_provider_id must be a valid UUID").nullable().optional(),
  status: z.enum(["active", "inactive", "maintenance"]).optional(),
});

export default defineEventHandler(async (event) => {
  await requireAdmin(event);
  const parsed = createRegionSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: parsed.error.issues[0]?.message || "Invalid request body" });
  }
  const body = parsed.data;

  const created = await withAdminDb(async (db) => {
    const existingSlug = await db.query.region.findFirst({
      where: eq(region.slug, body.slug),
    });
    if (existingSlug) {
      throw createError({ statusCode: 409, statusMessage: "Region slug is already in use" });
    }

    const [newRegion] = await db.insert(region).values({
      id: uuidv7(),
      slug: body.slug,
      display_name: body.display_name,
      s3_provider_id: body.s3_provider_id ?? null,
      status: body.status ?? "active",
    }).returning();

    return newRegion;
  });

  event.res.status = 201;
  return created;
});
