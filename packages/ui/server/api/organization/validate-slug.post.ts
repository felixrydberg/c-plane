import { organization } from "~~/server/schema";
import { eq } from "drizzle-orm";
import z from "zod";
import { getIdentityDb } from "~~/server/utils/db";

const validateSlugSchema = z.object({
  slug: z.string().trim().min(1, "Slug is required").transform((value) => value.toLowerCase()),
});

export default defineEventHandler(async (event) => {
  const parsed = validateSlugSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw createError({
      statusCode: 400,
      statusMessage: parsed.error.issues[0]?.message || "Invalid request body",
    });
  }

  const org = await getIdentityDb()
    .select({ id: organization.id })
    .from(organization)
    .where(eq(organization.slug, parsed.data.slug))
    .limit(1);

  return { exists: org.length > 0 };
});
