import { organization } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { getIdentityDb } from "~~/server/utils/db";

export default defineEventHandler(async (event) => {
  await requireSession(event);
  const body = await readBody<{ email?: string }>(event);
  const email = body?.email?.trim().toLowerCase();
  
  if (!email) {
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
    .where(eq(organization.email, email))
    .limit(1);

  return { exists: existingOrganization.length > 0 };
});
