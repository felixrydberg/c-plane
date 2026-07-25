import { signJWT, verifyJWT } from "better-auth/crypto"
import * as z from "zod"

const invalidRegistrationContext = "A username and valid email are required to create a passkey account"
const registrationContextSchema = z.object({
  name: z.string().trim().min(1).max(100),
  email: z.string().trim().toLowerCase().email(),
  scope: z.literal("passkey-registration"),
})

export function getPasskeyRegistrationContextSecret() {
  const secret = process.env.BETTER_AUTH_SECRET
  if (!secret) throw new Error("BETTER_AUTH_SECRET must be set for passkey registration")
  return secret
}

export async function createPasskeyRegistrationContext(identity: unknown) {
  const secret = getPasskeyRegistrationContextSecret()
  try {
    const { name, email } = registrationContextSchema.omit({ scope: true }).parse(identity)
    return await signJWT({ name, email, scope: "passkey-registration" }, secret, 300)
  } catch {
    throw new Error(invalidRegistrationContext)
  }
}

export async function parsePasskeyRegistrationContext(context: string | null | undefined, secret: string) {
  try {
    return registrationContextSchema.parse(await verifyJWT(context ?? "", secret))
  } catch {
    throw new Error(invalidRegistrationContext)
  }
}
