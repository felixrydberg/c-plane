import { createHash, randomBytes } from "node:crypto";

// Must match the Rust verifier in packages/api/src/middleware/auth.rs:
// unsalted hex(SHA-256) over the raw key.
export function hashApiKey(rawKey: string): string {
  return createHash("sha256").update(rawKey).digest("hex");
}

export function generateSecret(): string {
  return `ak_${randomBytes(32).toString("base64url")}`;
}
