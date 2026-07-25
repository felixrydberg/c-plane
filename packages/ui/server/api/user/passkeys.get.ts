import { getAuthenticatorName } from "@better-auth/passkey"
import { auth } from "~~/server/utils/auth"

export default defineEventHandler(async (event) => {
  const [passkeys, accounts] = await Promise.all([
    auth.api.listPasskeys({ headers: event.headers }),
    auth.api.listUserAccounts({ headers: event.headers }),
  ])

  return {
    passkeys: passkeys.map(({ id, name, createdAt, aaguid, deviceType }) => {
      const fallbackName = getAuthenticatorName(aaguid) ?? (deviceType === "multiDevice" ? "Synced passkey" : "Device passkey")

      return {
        id,
        name: name ?? `${fallbackName} · ${createdAt.toISOString().slice(0, 10)}`,
        createdAt,
      }
    }),
    hasAlternativeAuth: accounts.some(account => account.providerId !== "passkey"),
    hasPassword: accounts.some(account => account.providerId === "credential"),
  }
})
