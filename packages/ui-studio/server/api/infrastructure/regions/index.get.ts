import { requireSession } from "~~/server/utils/authorization";
import { db } from "~~/server/utils/auth";
import { region } from "~~/server/schema";

export default defineEventHandler(async (event) => {
  await requireSession(event);
  return db.select().from(region);
});
