import { defineEventHandler, proxyRequest } from 'h3'

export default defineEventHandler(async (event) => {
  const config = useRuntimeConfig(event)
  const { backendUrl = 'http://localhost:8080', apiKey, proxyPath = '/api/cplane' } =
    config.cplaneSdk ?? {}

  const rawPath = event.path.startsWith(proxyPath as string)
    ? event.path.slice((proxyPath as string).length)
    : event.path

  const backendPath = rawPath === '/health' ? rawPath : `/api${rawPath}`

  const [basePath, queryString] = backendPath.split('?')
  const cleanParams = new URLSearchParams(queryString ?? '')
  for (const [key, value] of [...cleanParams.entries()]) {
    if (!value) cleanParams.delete(key)
  }
  const query = cleanParams.toString()
  const cleanPath = query ? `${basePath}?${query}` : basePath

  const headers: Record<string, string> = {}
  if (apiKey) headers['x-api-key'] = apiKey as string

  return proxyRequest(event, `${backendUrl}${cleanPath}`, { headers })
})
