<script setup lang="ts">
import { ICONS } from '~/utils/icons'
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div>
      <UiPageEyebrow label="Observe" />
      <h1 class="mt-2 text-2xl font-semibold">Analytics</h1>
      <p class="text-muted text-sm mt-1">Monitor performance, usage, and trends across your organization.</p>
    </div>

    <div class="flex flex-col items-center justify-center py-16 gap-3 text-center border border-dashed border-default rounded-lg">
      <UIcon :name="ICONS.analytics" class="size-10 text-muted" />
      <p class="text-muted">Analytics data will appear here once you deploy services.</p>
    </div>
  </div>
</template>
