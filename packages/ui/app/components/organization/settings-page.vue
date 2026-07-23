<script setup lang="ts">
defineProps<{ title: string }>()
</script>

<template>
  <div class="mx-auto flex w-full max-w-6xl flex-col gap-8">
    <div class="border-b border-default pb-6">
      <p class="font-space-mono text-[11px] uppercase tracking-[0.08em] text-muted">Organization settings</p>
      <h1 class="mt-2 text-4xl font-normal tracking-[-0.04em]">{{ title }}</h1>
    </div>

    <OrganizationSettingsTabs />

    <slot />
  </div>
</template>
