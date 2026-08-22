import { organization } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { getIdentityDb } from "~~/server/utils/db";

export default defineEventHandler(async (event) => {
  const body = await readBody<{ email: string }>(event);
  
  if (!body.email || body.email.trim().length === 0) {
    throw createError({
      statusCode: 400,
      data: {
        message: "Email is required",
      },
    });
  }

  const existingOrganization = await getIdentityDb()
    .select()
    .from(organization)
    .where(eq(organization.email, body.email))
    .limit(1);

  return { exists: existingOrganization.length > 0 };
});
