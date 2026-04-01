import { organization } from "~~/server/schema";
import { eq } from "drizzle-orm";

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

  const org = await db.query.organization.findFirst({
    where: eq(organization.slug, body.slug),
  });

  return { exists: !!org };
});
