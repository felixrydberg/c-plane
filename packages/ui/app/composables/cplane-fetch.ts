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

export const useCplaneRequestFetch = () => {
  if (!import.meta.server) return rawFetch as unknown as AppFetch

  const requestFetch = useRequestFetch() as unknown as SimpleFetch
  const incoming = useRequestHeaders(['authorization', 'cookie'])
  const backendUrl = useRuntimeConfig().backendUrl

  const appFetch: SimpleFetch = (request, options) => {
    if (!isRustApiRequest(request)) return requestFetch(request, options)

    const headers = new Headers(options?.headers)
    for (const [name, value] of Object.entries(incoming)) {
      if (value && !headers.has(name)) headers.set(name, value)
    }

    return rawFetch(request, {
      ...options,
      baseURL: backendUrl,
      headers,
    })
  }

  return appFetch as unknown as AppFetch
}

export const cplaneFetch = ((request: string, options?: SimpleOptions) =>
  (useCplaneRequestFetch() as unknown as SimpleFetch)(request, options)) as unknown as AppFetch

const appUseFetch: SimpleUseFetch = (request, options = {}) => {
  const requestFetch = useCplaneRequestFetch()
  return (useFetch as unknown as SimpleUseFetch)(request, { ...options, $fetch: requestFetch })
}

export const useCplaneFetch = appUseFetch as unknown as typeof useFetch

const appUseLazyFetch: SimpleUseFetch = (request, options = {}) => {
  const requestFetch = useCplaneRequestFetch()
  return (useLazyFetch as unknown as SimpleUseFetch)(request, { ...options, $fetch: requestFetch })
}

export const useLazyCplaneFetch = appUseLazyFetch as unknown as typeof useLazyFetch
