import { createPasskeyRegistrationContext, passkeyRegistrationIdentitySchema } from "~~/server/utils/passkey-registration-context"

export default defineEventHandler(async (event) => {
  const identity = passkeyRegistrationIdentitySchema.safeParse(await readBody(event))
  if (!identity.success) {
    throw createError({ statusCode: 400, statusMessage: "A username and valid email are required to create a passkey account" })
  }

  return { context: await createPasskeyRegistrationContext(identity.data) }
})
