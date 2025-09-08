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
  runtimeConfig: {
    ORY_SDK_URL: process.env.NUXT_ORY_SDK_URL || "http://kratos:4433",
    public: {
      ORY_SDK_URL:
        process.env.NUXT_PUBLIC_ORY_SDK_URL || "http://127.0.0.1:4433",
    },
  },
  css: ['~/assets/css/main.css'],
  compatibilityDate: '2025-07-16',
  fonts: {
    families: [
      { name: "Space Mono", provider: "google" },
      { name: "Space Grotesk", provider: "google" },
      { name: "Audiowide", provider: "google" }
    ]
  }
});