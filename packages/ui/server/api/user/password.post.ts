import { auth } from "~~/server/utils/auth"

const compromisedPasswordMessage = "This password has been compromised in a data breach, please choose a different one."

// ponytail: Better Auth's server-only setPassword has no route path for the plugin to match; remove this when it exposes one.
async function assertPasswordIsNotCompromised(password: string) {
  const digest = await crypto.subtle.digest("SHA-1", new TextEncoder().encode(password))
  const hash = Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2, "0")).join("").toUpperCase()
  const response = await fetch(`https://api.pwnedpasswords.com/range/${hash.slice(0, 5)}`, {
    headers: {
      "Add-Padding": "true",
      "User-Agent": "C-Plane Password Checker",
    },
  })

  if (!response.ok) {
    throw createError({ statusCode: 503, statusMessage: "Unable to check password security. Please try again." })
  }

  const compromised = (await response.text())
    .split(/\r?\n/)
    .some(line => line.split(":")[0]?.toUpperCase() === hash.slice(5))

  if (compromised) {
    throw createError({ statusCode: 400, statusMessage: compromisedPasswordMessage })
  }
}

export default defineEventHandler(async (event) => {
  const body = await readBody<{ newPassword?: unknown }>(event)

  if (typeof body?.newPassword !== "string") {
    throw createError({ statusCode: 400, statusMessage: "New password is required" })
  }

  await assertPasswordIsNotCompromised(body.newPassword)

  return auth.api.setPassword({
    headers: event.headers,
    body: { newPassword: body.newPassword },
  })
})
