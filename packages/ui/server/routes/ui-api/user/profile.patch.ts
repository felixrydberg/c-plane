import { user } from "~~/server/schema";
import { getAuthDb } from "~~/server/utils/auth";
import { eq } from "drizzle-orm";
import z from "zod";

const profileSchema = z.object({
  name: z.string().trim().min(1, "Name is required"),
}).partial();

export default defineEventHandler(async (event) => {
  const session = await requireSession(event);
  const parsed = profileSchema.safeParse(await readBody(event));

  if (!parsed.success || !parsed.data.name) {
    throw createError({
      statusCode: 400,
      statusMessage: parsed.success ? "Name is required" : parsed.error.issues[0]?.message || "Invalid request body",
    });
  }

  const updated = await getAuthDb().update(user)
    .set({ name: parsed.data.name })
    .where(eq(user.id, session.user.id))
    .returning();

  if (updated.length === 0) {
    throw createError({
      statusCode: 404,
      message: "User not found"
    });
  }

  return updated[0];
});
