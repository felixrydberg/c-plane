import type { ClientType } from "~/utils/auth"

export type User = ClientType["$Infer"]["Session"]["user"]
export type Session = ClientType["$Infer"]["Session"]["session"]
