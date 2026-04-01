import { count, eq } from "drizzle-orm";
import { organization_member } from "~~/server/schema";

export default defineEventHandler(async (event) => {
  const session = await requireSession(event);
  

  const organization_count = await db
    .select({
      count: count(),
    })
    .from(organization_member)
    .where(eq(organization_member.user_id, session.user.id))
    .limit(1);

  return organization_count[0].count;
});
