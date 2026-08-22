type AppFetch = typeof $fetch
type SimpleOptions = Record<string, unknown> & { headers?: HeadersInit }
type SimpleFetch = (request: string, options?: SimpleOptions) => Promise<unknown>
type SimpleUseFetch = (request: unknown, options?: Record<string, unknown>) => unknown

const isRustApiRequest = (request: string) => {
  const path = request.startsWith('http://') || request.startsWith('https://')
    ? new URL(request).pathname
    : (request.split('?', 1)[0] ?? request)
  return path === '/api' || path.startsWith('/api/')
}

const rawFetch = $fetch as unknown as SimpleFetch

const appFetch: SimpleFetch = (request, options) => {
  if (!import.meta.server) return rawFetch(request, options)
  if (!isRustApiRequest(request)) {
    return (useRequestFetch() as unknown as SimpleFetch)(request, options)
  }

  const headers = new Headers(options?.headers)
  const incoming = useRequestHeaders(['authorization', 'cookie'])
  for (const [name, value] of Object.entries(incoming)) {
    if (value && !headers.has(name)) headers.set(name, value)
  }

  return rawFetch(request, {
    ...options,
    baseURL: useRuntimeConfig().backendUrl,
    headers,
  })
}

export const cplaneFetch = appFetch as unknown as AppFetch

const appUseFetch: SimpleUseFetch = (request, options = {}) =>
  (useFetch as unknown as SimpleUseFetch)(request, { ...options, $fetch: cplaneFetch })

export const useCplaneFetch = appUseFetch as unknown as typeof useFetch

const appUseLazyFetch: SimpleUseFetch = (request, options = {}) =>
  (useLazyFetch as unknown as SimpleUseFetch)(request, { ...options, $fetch: cplaneFetch })

export const useLazyCplaneFetch = appUseLazyFetch as unknown as typeof useLazyFetch
