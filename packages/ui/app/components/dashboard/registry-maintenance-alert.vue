<script setup lang="ts">
const props = defineProps<{ organizationId: string }>()

const endpoint = computed(() =>
  `/api/cplane/organization/${props.organizationId as ':organization_id'}/registry/maintenance` as const
)
const { data: maintenance, refresh } = await useFetch(endpoint)

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
      :description="`The Registry is read-only while garbage collection is ${maintenance.phase}. Pulls remain available.`"
      class="border border-warning/40 bg-warning/15 text-warning-800 dark:border-warning-400/40 dark:bg-warning-950/40 dark:text-warning-200"
    />
  </div>
</template>
