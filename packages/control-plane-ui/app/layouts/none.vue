<script setup lang="ts">
  import * as locales from '@nuxt/ui/locale'
  const locale = ref('en');
</script>

<template>
  <div>
    <slot />
  </div>
</template>
