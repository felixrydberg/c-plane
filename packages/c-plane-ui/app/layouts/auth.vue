<script setup lang="ts">
  import * as locales from '@nuxt/ui/locale'
  const locale = ref('en');
</script>

<!-- eslint-disable vue/no-multiple-template-root -->
<template>
  <header class="fixed h-16 w-full">
    <UContainer class="h-full flex items-end justify-between">
    </UContainer>
  </header>
  <UMain>
    <slot />
  </UMain>
  <footer>
    <UContainer class="h-16 flex items-center justify-center text-sm text-neutral-400">
      &copy; 2024 Cloud44. All rights reserved.
      <UColorModeButton />
      <ULocaleSelect v-model="locale" :locales="Object.values(locales)" class="w-48" />
    </UContainer>
  </footer>
</template>
