// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  devtools: { enabled: true },
  modules: [
    '@nuxt/ui',
    '@nuxt/eslint',
    '@nuxt/test-utils',
    '@nuxt/image'
  ],
  css: ['~/assets/css/main.css'],
  compatibilityDate: '2025-07-16',
  ui: {
    colorMode: false,
  },
  fonts: {
    families: [
      { name: "Space Mono", provider: "google" },
      { name: "Space Grotesk", provider: "google" },
      { name: "Audiowide", provider: "google" }
    ]
  }
});
