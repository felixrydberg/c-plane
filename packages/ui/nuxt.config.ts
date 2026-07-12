// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  runtimeConfig: {
    controlPlaneUrl: '',
    cplaneServiceToken: '',
  },
  devtools: { enabled: true },
  devServer: {
    host: '0.0.0.0'
  },
  modules: [
    '@nuxt/ui',
    '@nuxt/eslint',
    '@nuxt/image',
    "@pinia/nuxt",
    "@polar-sh/nuxt",
  ],
  css: ['~/assets/css/main.css'],
  compatibilityDate: '2025-07-16',
  vite: {
    optimizeDeps: {
      include: ['qrcode'],
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
    minify: true,
    sourceMap: false,
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
    provider: 'server',
    collections: ['heroicons', 'heroicons-solid', 'lucide'],
    clientBundle: {
      scan: {
        globInclude: [
          'app/**/*.{vue,ts,tsx,md,mdc,mdx,js,jsx}',
          'packages/**/app/**/*.{vue,ts,tsx,md,mdx,js,jsx}',
          'node_modules/@nuxt/ui/**/*.{vue,ts,tsx,js,jsx}',
          'node_modules/@id-proval/ui-shared/**/*.{vue,ts,tsx,js,jsx}',
        ],
      },
      sizeLimitKb: 512,
    },
  },
});
