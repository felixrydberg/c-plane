<script setup lang="ts">
import { ICONS } from '~/utils/icons'
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div>
      <UiPageEyebrow label="Observe" />
      <h1 class="text-2xl font-semibold">Logs</h1>
      <p class="text-muted text-sm mt-1">View real-time and historical logs across your organization.</p>
    </div>

    <div class="flex flex-col items-center justify-center py-16 gap-3 text-center border border-dashed border-default rounded-lg">
      <UIcon :name="ICONS.logs" class="size-10 text-muted" />
      <p class="text-muted">Logs will appear here once you deploy services.</p>
    </div>
  </div>
</template>
