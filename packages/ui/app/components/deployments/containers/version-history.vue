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

const route = useRoute()
const router = useRouter()

const endpoint = computed(() => `/api/organization/${props.organizationId as ':organization_id'}/containers/${props.containerId as ':container_id'}/history` as const)

const cursors = ref<Array<string | undefined>>([undefined])
const page = ref(0)
const activeCursor = computed(() => cursors.value[page.value])

defineExpose({ refresh: reset })

const { data: history, status, error, refresh } = await useCplaneFetch(endpoint, {
  key: () => `container-history-${props.containerId}-${activeCursor.value ?? 'first'}`,
  query: computed(() => ({
    environment_id: props.environmentId,
    timeline_id: props.timelineId,
    limit: 10,
    ...(activeCursor.value ? { cursor: activeCursor.value } : {}),
  })),
})

const entries = computed(() => history.value?.data ?? [])
const nextCursor = computed(() => history.value?.next_cursor ?? undefined)
const selectedEntryVisible = computed(() => page.value === 0 && entries.value.some(entry => entry.revision_id === props.timelineId))
const historyPageLabel = computed(() => {
  const revisionNumbers = entries.value.map(entry => entry.revision_number)
  if (!revisionNumbers.length) return `Page ${page.value + 1}`
  const oldest = Math.min(...revisionNumbers)
  const newest = Math.max(...revisionNumbers)
  return `Page ${page.value + 1} · revisions ${oldest}–${newest}`
})

async function reset() {
  cursors.value = [undefined]
  if (page.value === 0) {
    await refresh()
  } else {
    page.value = 0
  }
}

watch(() => [props.timelineId, props.environmentId], () => {
  void reset()
})

function older() {
  const next = nextCursor.value
  if (!next) return
  if (cursors.value.length <= page.value + 1) {
    cursors.value.push(next)
  } else {
    cursors.value[page.value + 1] = next
  }
  page.value += 1
}

function newer() {
  if (page.value > 0) page.value -= 1
}

async function viewRevision(revisionId: string) {
  await router.replace({ query: { ...route.query, revision: revisionId } })
}

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
  <aside class="min-w-0 border-t border-default/60 px-5 py-6 xl:border-l xl:border-t-0" aria-label="Revision history">
    <h2 class="text-sm font-semibold">Revision History</h2>
    <p class="mt-1 text-xs text-muted">Changes to this container along the selected revision's history.</p>
    <p v-if="page === 0 && !selectedEntryVisible && !error" class="mt-2 text-xs text-muted">This revision keeps the same container configuration. Showing earlier changes.</p>
    <p v-else-if="page > 0 && !error" class="mt-2 text-xs text-muted">Showing earlier changes. The selected revision is on page 1.</p>

    <div v-if="status === 'pending'" role="status" class="flex items-center gap-2 py-8 text-sm text-muted">
      <UIcon name="i-lucide-loader-circle" class="size-4 animate-spin" />
      Loading history&hellip;
    </div>
    <div v-else-if="error" role="alert" class="space-y-3 py-8">
      <p class="text-sm text-muted">Could not load revision history.</p>
      <UButton :icon="ICONS.refresh" color="neutral" size="sm" @click="reset()">Retry</UButton>
    </div>
    <p v-else-if="!entries.length" class="py-8 text-sm text-muted">No configuration changes in this range.</p>
    <ol v-else class="mt-6 space-y-6">
      <li v-for="(entry, index) in entries" :key="`${entry.revision_id}-${index}`" class="min-w-0">
        <button
          type="button"
          class="group block w-full rounded-md px-2 py-1.5 text-left transition-colors hover:bg-elevated/40 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          :aria-current="entry.revision_id === props.timelineId ? 'page' : undefined"
          @click="viewRevision(entry.revision_id)"
        >
          <div class="flex flex-wrap items-center gap-2">
            <h3 class="text-sm font-medium">Revision {{ entry.revision_number }}</h3>
            <span v-if="page === 0 && entry.revision_id === props.timelineId" class="rounded bg-elevated px-1.5 py-0.5 text-[11px] font-medium">Selected</span>
            <UIcon :name="ICONS.chevronRight" class="ml-auto size-3.5 text-muted opacity-0 transition-opacity group-hover:opacity-100" aria-hidden="true" />
          </div>
          <NuxtTime :datetime="entry.created_at" relative class="mt-1 block text-xs text-muted" />
          <p class="mt-2 text-sm text-muted">{{ summary(entry) }}</p>
        </button>
      </li>
    </ol>

    <div v-if="status !== 'pending' && !error" class="mt-6 flex justify-center">
      <nav aria-label="Revision history pages" class="inline-flex items-center rounded-lg border border-default/60 bg-elevated/20 p-1 shadow-sm">
        <UButton
          color="neutral"
          variant="solid"
          size="sm"
          :leading-icon="ICONS.chevronLeft"
          class="min-w-22 justify-center"
          :disabled="page === 0"
          @click="newer()"
        >
          Previous
        </UButton>
        <span aria-current="page" class="mx-1 min-w-18 rounded-md border border-default/60 bg-default/10 px-3 py-1.5 text-center text-xs font-medium text-muted">
          {{ historyPageLabel }}
        </span>
        <UButton
          color="neutral"
          variant="solid"
          size="sm"
          :trailing-icon="ICONS.chevronRight"
          class="min-w-18 justify-center"
          :disabled="!nextCursor"
          @click="older()"
        >
          Next
        </UButton>
      </nav>
    </div>
  </aside>
</template>
