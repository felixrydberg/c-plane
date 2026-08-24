<script setup lang="ts">
const endpoint = '/api/registry/maintenance' as const
const { data: maintenance, refresh } = await useCplaneFetch(endpoint)

let refreshTimer: ReturnType<typeof setInterval> | undefined
onMounted(() => {
  refreshTimer = setInterval(refresh, 5000)
})
onBeforeUnmount(() => clearInterval(refreshTimer))
</script>

<template>
  <div v-if="maintenance?.read_only" class="shrink-0 bg-default px-6 py-3 lg:px-8">
    <UAlert
      color="warning"
      variant="subtle"
      icon="i-heroicons:exclamation-triangle"
      title="Registry maintenance in progress"
      :description="`New Registry writes are blocked while garbage collection is ${maintenance.phase}; in-flight writes may finish. The Registry is read-only during collecting; pulls remain available.`"
      class="border border-warning/40 bg-warning/15 text-warning-800 dark:border-warning-400/40 dark:bg-warning-950/40 dark:text-warning-200"
    />
  </div>
</template>
