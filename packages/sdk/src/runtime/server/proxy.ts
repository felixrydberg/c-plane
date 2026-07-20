import { defineEventHandler, proxyRequest } from 'h3'

export default defineEventHandler(async (event) => {
  const config = useRuntimeConfig(event)
  const { backendUrl = 'http://localhost:8080', apiKey, proxyPath = '/api/cplane' } =
    config.cplaneSdk ?? {}

  const path = event.path.replace(new RegExp(`^${escapeRegex(proxyPath)}`), '')
  const backendPath = path === '/health' ? path : `/api${path}`

  const [basePath, queryString] = backendPath.split('?')
  const cleanParams = new URLSearchParams(queryString ?? '')
  for (const [key, value] of cleanParams.entries()) {
    if (!value) cleanParams.delete(key)
  }
  const query = cleanParams.toString()
  const cleanPath = query ? `${basePath}?${query}` : basePath

  const headers: Record<string, string> = {}
  if (apiKey) headers['x-api-key'] = apiKey as string

  return proxyRequest(event, `${backendUrl}${cleanPath}`, { headers })
})

function escapeRegex(s: string) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}
