import { user } from "~~/server/schema";
import { getAuthDb } from "~~/server/utils/auth";
import { eq } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const session = await requireSession(event);
  const body = await readBody(event);

  if (typeof body.name === "string" && body.name.trim()) {
    const updated = await getAuthDb().update(user)
      .set({ name: body.name.trim() })
      .where(eq(user.id, session.user.id))
      .returning();

    if (updated.length === 0) {
      throw createError({
        statusCode: 404,
        message: "User not found"
      });
    }

    return updated[0];
  }

  throw createError({ statusCode: 400, message: "Name is required" });
});
