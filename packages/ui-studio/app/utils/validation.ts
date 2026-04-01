import * as z from 'zod'

export const PASSWORD_MIN_LENGTH = 8

export const passwordSchema = z.string()
  .min(PASSWORD_MIN_LENGTH, `Password must be at least ${PASSWORD_MIN_LENGTH} characters`)

export const passwordConfirmationSchema = z.object({
  password: passwordSchema,
  confirmPassword: z.string().min(PASSWORD_MIN_LENGTH, `Password must be at least ${PASSWORD_MIN_LENGTH} characters`)
}).refine((data) => data.confirmPassword === data.password, {
  message: "Passwords don't match",
  path: ['confirmPassword']
})

export const changePasswordSchema = z.object({
  currentPassword: z.string().min(PASSWORD_MIN_LENGTH, 'Password is required'),
  newPassword: passwordSchema,
  confirmPassword: z.string().min(PASSWORD_MIN_LENGTH, `Password must be at least ${PASSWORD_MIN_LENGTH} characters`),
}).refine((data) => data.newPassword === data.confirmPassword, {
  message: "Passwords don't match",
  path: ["confirmPassword"],
})
