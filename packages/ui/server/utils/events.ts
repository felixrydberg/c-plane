import { event } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import type { event_types } from "~~/server/schema";
import { uuidv7 } from "uuidv7";

export async function logEvent(
  organization_id: string,
  type: typeof event_types.enumValues[number],
  payload: Record<string, unknown>,
  system: boolean = false
) {
  try {
    await withTenantDb([organization_id], (db) => {
      return db.insert(event).values({
        id: uuidv7(),
        organization_id,
        type,
        payload,
        system,
        created_at: new Date(),
      });
    });
  } catch (error) {
    // Don't throw - events are best-effort
    console.error("Failed to log event:", error);
  }
}
