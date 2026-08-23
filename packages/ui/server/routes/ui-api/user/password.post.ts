import { auth } from "~~/server/utils/auth"
import * as z from "zod"

const compromisedPasswordMessage = "This password has been compromised in a data breach, please choose a different one."
const newPasswordSchema = z.object({
  newPassword: z.string().min(8, "Password must be at least 8 characters"),
})

// ponytail: Better Auth's server-only setPassword has no route path for the plugin to match; remove this when it exposes one.
async function assertPasswordIsNotCompromised(password: string) {
  const digest = await crypto.subtle.digest("SHA-1", new TextEncoder().encode(password))
  const hash = Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2, "0")).join("").toUpperCase()
  let range: string
  try {
    range = await $fetch<string>(`https://api.pwnedpasswords.com/range/${hash.slice(0, 5)}`, {
      headers: {
        "Add-Padding": "true",
        "User-Agent": "C-Plane Password Checker",
      },
      timeout: 5_000,
      retry: 0,
    })
  } catch {
    throw createError({ statusCode: 503, statusMessage: "Unable to check password security. Please try again." })
  }

  const compromised = range
    .split(/\r?\n/)
    .some(line => {
      const [suffix, count] = line.split(":")
      return suffix?.toUpperCase() === hash.slice(5) && Number.parseInt(count ?? "", 10) > 0
    })

  if (compromised) {
    throw createError({ statusCode: 400, statusMessage: compromisedPasswordMessage })
  }
}

export default defineEventHandler(async (event) => {
  const body = newPasswordSchema.safeParse(await readBody(event))

  if (!body.success) {
    throw createError({ statusCode: 400, statusMessage: "New password must be at least 8 characters" })
  }

  await assertPasswordIsNotCompromised(body.data.newPassword)

  return auth.api.setPassword({
    headers: event.headers,
    body: { newPassword: body.data.newPassword },
  })
})
