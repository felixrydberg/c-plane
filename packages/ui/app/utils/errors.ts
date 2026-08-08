import { FetchError } from 'ofetch'

type ErrorResponse = {
  data?: { message?: unknown; statusMessage?: unknown }
  message?: unknown
}

export function getErrorMessage(error: unknown, fallback: string): string {
  if (typeof error === 'string' && error.trim()) return error
  const response = (error && typeof error === 'object' ? error : {}) as ErrorResponse
  const message = response.data?.message ?? response.data?.statusMessage
  if (typeof message === 'string' && message.trim()) return message
  if (!(error instanceof FetchError) && typeof response.message === 'string' && response.message.trim()) {
    return response.message
  }
  return fallback
}
