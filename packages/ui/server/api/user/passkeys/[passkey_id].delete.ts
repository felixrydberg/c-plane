import { auth } from "~~/server/utils/auth"

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, "passkey_id")
  if (!id) {
    throw createError({ statusCode: 400, statusMessage: "Passkey id is required" })
  }

  const [passkeys, accounts] = await Promise.all([
    auth.api.listPasskeys({ headers: event.headers }),
    auth.api.listUserAccounts({ headers: event.headers }),
  ])
  const targetExists = passkeys.some(passkey => passkey.id === id)
  const hasAlternativeAuth = accounts.some(account => account.providerId !== "passkey")

  if (!targetExists) {
    throw createError({ statusCode: 404, statusMessage: "Passkey not found" })
  }

  if (passkeys.length === 1 && !hasAlternativeAuth) {
    throw createError({
      statusCode: 409,
      statusMessage: "Add another sign-in method before removing your last passkey",
    })
  }

  return auth.api.deletePasskey({
    headers: event.headers,
    body: { id },
  })
})
