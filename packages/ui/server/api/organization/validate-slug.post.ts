import { organization } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { getIdentityDb } from "~~/server/utils/db";

export default defineEventHandler(async (event) => {
  const body = await readBody<{ slug: string }>(event);
  
  if (!body.slug || body.slug.trim().length === 0) {
    throw createError({
      statusCode: 400,
      data: {
        message: "Slug is required",
      },
    });
  }

  const org = await getIdentityDb()
    .select({ id: organization.id })
    .from(organization)
    .where(eq(organization.slug, body.slug))
    .limit(1);

  return { exists: org.length > 0 };
});
