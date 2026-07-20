import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { addServerHandler, addTypeTemplate, defineNuxtModule } from '@nuxt/kit'

const __dirname = fileURLToPath(new URL('.', import.meta.url))

export interface CplaneSdkOptions {
  /** Path prefix for the proxy routes and useFetch autocomplete. Default: '/api/cplane' */
  proxyPath?: string
  /** Optional API key for backend auth. When unset, cookies are forwarded for session auth. */
  apiKey?: string
  /** Backend URL to proxy to. Default: process.env.BACKEND_URL ?? 'http://localhost:8080' */
  backendUrl?: string
}

const methods = new Set(['get', 'post', 'put', 'patch', 'delete', 'options', 'head', 'trace'])
const spec = JSON.parse(readFileSync(new URL('../openapi.json', import.meta.url), 'utf8')) as {
  paths: Record<string, Record<string, unknown>>
}

export default defineNuxtModule<CplaneSdkOptions>({
  meta: {
    name: '@cplane/sdk/nuxt',
    configKey: 'cplaneSdk',
  },
  defaults: {
    proxyPath: '/api/cplane',
  },
  setup(options, nuxt) {
    const proxyPath = options.proxyPath!

    // Inject runtime config so the proxy handler can read it at runtime
    nuxt.options.runtimeConfig.cplaneSdk = {
      backendUrl: options.backendUrl ?? process.env.BACKEND_URL ?? 'http://localhost:8080',
      apiKey: options.apiKey,
      proxyPath,
    }

    // Build route map from OpenAPI spec for type template
    const routePath = (path: string) =>
      `${proxyPath}${path.replace(/^\/api/, '').replace(/\{([^}]+)\}/g, ':$1')}`

    const routes = Object.entries(spec.paths).flatMap(([path, pathItem]) =>
      Object.entries(pathItem)
        .filter(([method]) => methods.has(method))
        .map(([method]) => ({
          method,
          openApiPath: path,
          publicPath: routePath(path),
        })),
    )

    const routeGroups = new Map<string, typeof routes>()
    for (const route of routes) {
      const group = routeGroups.get(route.publicPath) ?? []
      group.push(route)
      routeGroups.set(route.publicPath, group)
    }

    const typeTemplate = () => `import type { paths } from '@cplane/sdk'

type JsonBody<Response> = Response extends { content: infer Content }
  ? Content extends { 'application/json': infer Body } ? Body : undefined
  : undefined

type SuccessStatus = 200 | 201 | 202 | 203 | 204 | 205 | 206 | 207 | 208 | 226

type SuccessBody<Responses> = Responses extends object
  ? JsonBody<{
      [Status in keyof Responses]: Status extends SuccessStatus ? Responses[Status] : never
    }[keyof Responses]>
  : undefined

type CplaneResponse<Path extends keyof paths, Method extends keyof paths[Path]> =
  NonNullable<paths[Path][Method]> extends { responses: infer Responses }
    ? SuccessBody<Responses>
    : undefined

declare module 'nitropack/types' {
  interface InternalApi {
${[...routeGroups].map(([publicPath, operations]) => `    ${JSON.stringify(publicPath)}: {
${operations.map(({ openApiPath, method }) => `      ${method}: CplaneResponse<${JSON.stringify(openApiPath)}, '${method}'>`).join('\n')}
    }`).join('\n')}
  }
}

export {}
`

    addTypeTemplate({
      filename: 'types/cplane-sdk.d.ts',
      getContents: typeTemplate,
    })

    // Register catch-all proxy handler at the configured path
    addServerHandler({
      route: `${proxyPath}/**`,
      handler: resolve(__dirname, 'runtime/server/proxy.ts'),
    })
  },
})
