<script setup lang="ts">
import type { components } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'

type HistoryEntry = components['schemas']['ContainerHistoryEntry']

const props = defineProps<{
  organizationId: string
  containerId: string
  environmentId: string
  timelineId: string
}>()

const endpoint = computed(() => `/api/organization/${props.organizationId as ':organization_id'}/containers/${props.containerId as ':container_id'}/history` as const)
defineExpose({ refresh: () => refresh() })
const { data: history, status, error, refresh } = await useCplaneFetch(endpoint, {
  query: computed(() => ({ environment_id: props.environmentId, timeline_id: props.timelineId })),
})

const fieldLabels: Record<string, string> = {
  image: 'Image',
  resolved_image: 'Image digest',
  external_registry_id: 'External registry',
  replica_count: 'Replicas',
  port: 'Port',
  public: 'Public access',
  cpu: 'CPU',
  memory: 'Memory',
  health_check: 'Health check',
}

function fieldLabel(field: string): string {
  return field.startsWith('env.') ? 'Environment variables' : (fieldLabels[field] ?? field)
}

function summary(entry: HistoryEntry): string {
  if (entry.baseline === 'initial') return 'Initial configuration'
  if (entry.baseline === 'earliest_available') return 'Earliest available configuration'
  if (!entry.changes.length) return 'No configuration changes'
  const labels = [...new Set(entry.changes.map(change => fieldLabel(change.field)))]
  return `${labels.join(', ')} changed`
}

</script>

<template>
  <aside class="min-w-0 border-t border-default/60 px-5 py-6 xl:border-l xl:border-t-0" aria-label="Version history">
    <h2 class="text-sm font-semibold">Version History</h2>
    <p class="mt-1 text-xs text-muted">Changes leading to the selected version.</p>

    <div v-if="status === 'pending'" role="status" class="flex items-center gap-2 py-8 text-sm text-muted">
      <UIcon name="i-lucide-loader-circle" class="size-4 animate-spin" />
      Loading history&hellip;
    </div>
    <div v-else-if="error" role="alert" class="space-y-3 py-8">
      <p class="text-sm text-muted">Could not load version history.</p>
      <UButton :icon="ICONS.refresh" color="neutral" size="sm" @click="refresh()">Retry</UButton>
    </div>
    <p v-else-if="!history?.length" class="py-8 text-sm text-muted">No version history available.</p>
    <ol v-else class="mt-6 space-y-6">
      <li v-for="(entry, index) in history" :key="`${entry.id}-${index}`" class="min-w-0">
        <div class="flex flex-wrap items-center gap-2">
          <h3 class="text-sm font-medium">Version {{ entry.version }}</h3>
          <span v-if="index === 0" class="rounded bg-elevated px-1.5 py-0.5 text-[11px] font-medium">Selected</span>
        </div>
        <NuxtTime :datetime="entry.created_at" relative class="mt-1 block text-xs text-muted" />
        <p class="mt-2 text-sm text-muted">{{ summary(entry) }}</p>
      </li>
    </ol>
  </aside>
</template>
