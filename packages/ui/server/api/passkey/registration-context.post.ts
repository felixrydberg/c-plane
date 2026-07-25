import { createPasskeyRegistrationContext } from "~~/server/utils/passkey-registration-context"

export default defineEventHandler(async (event) => {
  return { context: await createPasskeyRegistrationContext(await readBody(event)) }
})
