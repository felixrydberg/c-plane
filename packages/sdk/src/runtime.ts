import createClient from 'openapi-fetch'
import type { paths } from './generated.ts'

export type SdkOptions = {
  baseUrl?: string
  headers?: HeadersInit
  fetch?: typeof fetch
}

export const createRawClient = (options: SdkOptions = {}) =>
  createClient<paths>({
    baseUrl: options.baseUrl ?? '',
    credentials: 'include',
    headers: options.headers,
    fetch: options.fetch,
  })
