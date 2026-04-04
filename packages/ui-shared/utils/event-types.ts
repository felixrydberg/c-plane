export const EVENT_TYPE_VALUES = [
  "organization:member_added",
  "organization:member_removed",
  "organization:invitation_created",
  "organization:invitation_accepted",
  "organization:invitation_revoked",
  "organization:invitation_declined",
  "api-key:created",
  "api-key:revoked",
  "api-key:updated",
  "api-key:rolled",
  "verification:created",
  "verification:completed",
] as const;

export type EventType = (typeof EVENT_TYPE_VALUES)[number];
