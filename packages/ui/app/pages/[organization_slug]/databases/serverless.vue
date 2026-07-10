<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const store = useStore();
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div>
      <h1 class="text-2xl font-semibold">Serverless Databases</h1>
      <p class="text-muted text-sm mt-1">
        Manage serverless Postgres databases across all projects.
      </p>
    </div>

    <div class="flex flex-col items-center justify-center py-16 gap-3 text-center border border-dashed border-default rounded-lg">
      <UIcon :name="ICONS.globeAlt" class="size-10 text-muted" />
      <p class="text-lg font-medium">Coming Soon</p>
      <p class="text-muted">Serverless databases are not yet available.</p>
    </div>
  </div>
</template>
