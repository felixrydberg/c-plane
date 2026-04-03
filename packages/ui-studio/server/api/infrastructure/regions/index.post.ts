import { requireSession } from "~~/server/utils/authorization";
import { db } from "~~/server/utils/auth";
import { region } from "~~/server/schema";
import { uuidv7 } from "uuidv7";
import { z } from "zod";

const createRegionSchema = z.object({
  slug: z.string().trim().min(1, "slug is required"),
  display_name: z.string().trim().min(1, "display_name is required"),
  status: z.enum(["active", "inactive", "maintenance"]).optional(),
});

export default defineEventHandler(async (event) => {
  await requireSession(event);
  const parsed = createRegionSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: parsed.error.issues[0]?.message || "Invalid request body" });
  }
  const body = parsed.data;

  const [created] = await db.insert(region).values({
    id: uuidv7(),
    slug: body.slug,
    display_name: body.display_name,
    status: body.status ?? "active",
  }).returning();

  event.res.status = 201;
  return created;
});
