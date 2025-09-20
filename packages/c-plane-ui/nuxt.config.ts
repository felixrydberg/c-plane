// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  devtools: { enabled: true },
  modules: [
    '@nuxt/ui',
    '@nuxt/eslint',
    '@nuxt/test-utils',
    '@nuxt/image',
    "@pinia/nuxt"
  ],
  css: ['~/assets/css/main.css'],
  compatibilityDate: '2025-07-16',
  fonts: {
    families: [
      { name: "Space Mono", provider: "google" },
      { name: "Space Grotesk", provider: "google" },
      { name: "Audiowide", provider: "google" }
    ]
  },
  icon: {
    clientBundle: {
      scan: true,
      sizeLimitKb: 512,
    },
  },
});
