export const API_KEY_SCOPE_VALUES = [
  "read:sessions",
  "write:sessions",
] as const;

export type ApiKeyScope = (typeof API_KEY_SCOPE_VALUES)[number];
