// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  devtools: { enabled: true },
  devServer: {
    host: '0.0.0.0'
  },
  modules: [
    '@nuxt/ui',
    '@nuxt/eslint',
    '@nuxt/test-utils',
    '@nuxt/image',
    "@pinia/nuxt",
    "@polar-sh/nuxt",
  ],
  css: ['~/assets/css/main.css'],
  compatibilityDate: '2025-07-16',
  vite: {
    optimizeDeps: {
      exclude: ['@ffmpeg/ffmpeg', '@ffmpeg/util']
    },
    worker: {
      format: 'es'
    },
    server: {
      origin: 'https://cplane.240284308.xyz',
      allowedHosts: [
        'cplane.240284308.xyz',
      ],
      headers: {
        'Access-Control-Allow-Origin': '*',
        'Access-Control-Allow-Methods': 'GET, POST, PUT, PATCH, DELETE, OPTIONS',
        'Access-Control-Allow-Headers': 'Content-Type, Authorization',
        'Cache-Control': 'no-store, no-cache, must-revalidate, proxy-revalidate',
      },
      cors: true,
    }
  },
  nitro: {
    preset: 'bun',
    routeRules: {
      '/api/**': {
        cors: true,
        headers: {
          'Access-Control-Allow-Origin': '*',
          'Access-Control-Allow-Methods': 'GET, POST, PUT, PATCH, DELETE, OPTIONS',
          'Access-Control-Allow-Headers': 'Content-Type, Authorization',
          'Cache-Control': 'no-store, no-cache, must-revalidate, proxy-revalidate',
        },
      },
    },
    experimental: {
      openAPI: true,
      websocket: true,
    },
    esbuild: {
      options: {
        target: "esnext"
      }
    },
  },
  experimental: {
    crossOriginPrefetch: false,
  },
  fonts: {
    families: [
      { name: "Space Mono", provider: "google" },
      { name: "Space Grotesk", provider: "google" },
      { name: "Audiowide", provider: "google" }
    ]
  },
  icon: {
    provider: 'none',
    collections: ['heroicons', 'heroicons-solid', 'lucide'],
    clientBundle: {
      icons: ['lucide:x', 'lucide:chevron-down', 'lucide:check', 'lucide:loader-circle'],
      scan: {
        globInclude: ['app/**/*.{vue,ts,tsx,md,mdc,mdx}'],
      },
      sizeLimitKb: 512,
    },
  },
});
