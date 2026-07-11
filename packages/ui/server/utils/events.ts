import { event } from "~~/server/schema";
import { withTenantDb } from "~~/server/utils/db";
import type { EventType } from "~~/server/schema";
import { uuidv7 } from "uuidv7";

type EventScope = {
  project_id?: string;
  actor_id?: string;
};

type TxParam = Parameters<Parameters<typeof withTenantDb>[1]>[0];

export async function logEvent(
  organization_id: string,
  type: EventType,
  payload: Record<string, unknown>,
  system: boolean = false,
  scope: EventScope = {},
  tx?: TxParam,
) {
  const insert = (db: TxParam) =>
    db.insert(event).values({
      id: uuidv7(),
      organization_id,
      type,
      payload,
      system,
      ...scope,
      created_at: new Date(),
    });

  if (tx) {
    return insert(tx);
  }
  return withTenantDb([organization_id], insert);
}
