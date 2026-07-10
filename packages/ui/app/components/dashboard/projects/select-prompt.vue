<script setup lang="ts">
import { ICONS } from '~/utils/icons'
</script>

<template>
  <div class="flex flex-col items-center justify-center py-24 gap-4 text-center w-full mx-auto max-w-6xl">
    <UIcon :name="ICONS.folder" class="size-12 text-muted" />
    <h2 class="text-lg font-medium">Select a project</h2>
    <p class="text-muted text-sm">Choose a project from the selector above to continue.</p>
  </div>
</template>
