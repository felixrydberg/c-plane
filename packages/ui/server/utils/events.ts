import { event } from "~~/server/schema";
import { db } from "~~/server/utils/auth";
import type { event_types } from "~~/server/schema";
import { uuidv7 } from "uuidv7";

export async function logEvent(
  organization_id: string,
  type: typeof event_types.enumValues[number],
  payload: Record<string, unknown>,
  system: boolean = false
) {
  try {
    await db.insert(event).values({
      id: uuidv7(),
      organization_id,
      type,
      payload,
      system,
      created_at: new Date(),
    });
  } catch (error) {
    console.error("Failed to log event:", error);
    // Don't throw - events are best-effort
  }
}
