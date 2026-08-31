<script setup lang="ts">
defineProps<{ title: string }>()
</script>

<template>
  <div class="mx-auto flex w-full max-w-6xl flex-col gap-8">
    <div class="border-b border-default pb-6">
      <UiPageEyebrow label="Manage organization" />
      <h1 class="mt-1 text-2xl font-semibold">{{ title }}</h1>
    </div>

    <slot />
  </div>
</template>
