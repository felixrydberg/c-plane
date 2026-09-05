<script setup lang="ts">
import { h } from 'vue'
import type { TableColumn } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

interface AuditEvent {
  id: string
  action: string
  summary: string
  actor_id: string | null
  actor_name: string | null
  created_at: string
}

const store = useStore()

if (!store.organization?.id) {
  throw createError('Organization not found in store')
}

const NuxtTime = resolveComponent('NuxtTime')
const endpoint = `/api/organization/${store.organization.id as ':organization_id'}/events` as const
const { data: events, status } = await useCplaneFetch<AuditEvent[]>(endpoint, {
  query: { limit: 50 },
  default: () => [],
})

const AUDIT_PAGE_SIZE = 20
const offset = ref(0)

const auditRange = ref('7')
const auditRangeItems = [
  { label: 'Last 7 days', value: '7' },
  { label: 'Last 30 days', value: '30' },
  { label: 'All activity', value: 'all' },
]
const auditAction = ref('all')

const auditFilterItems = computed(() => [[
  {
    label: 'All actions',
    onSelect: () => { auditAction.value = 'all' },
  },
  ...Array.from(new Set(events.value.map(event => event.action))).map(action => ({
    label: action.charAt(0).toUpperCase() + action.slice(1),
    onSelect: () => { auditAction.value = action },
  })),
]])

const filteredEvents = computed(() => {
  const rangeCutoff = auditRange.value === 'all'
    ? 0
    : Date.now() - Number(auditRange.value) * 24 * 60 * 60 * 1000

  return events.value.filter(event => {
    const matchesRange = rangeCutoff === 0 || new Date(event.created_at).getTime() >= rangeCutoff
    const matchesAction = auditAction.value === 'all' || event.action === auditAction.value
    return matchesRange && matchesAction
  })
})

const paginatedEvents = computed(() => filteredEvents.value.slice(offset.value, offset.value + AUDIT_PAGE_SIZE))
const showPagination = computed(() => filteredEvents.value.length > AUDIT_PAGE_SIZE)

watch([auditRange, auditAction], () => {
  offset.value = 0
})

const formatDate = (value: string) => new Date(value).toLocaleDateString('sv-SE')

const columns: TableColumn<AuditEvent>[] = [
  {
    accessorKey: 'created_at',
    header: 'Action Time ↓',
    cell: item => h('div', { class: 'flex items-baseline gap-2 whitespace-nowrap' }, [
      h('span', { class: 'text-sm text-default' }, formatDate(item.row.original.created_at)),
      h(NuxtTime, {
        class: 'font-mono text-xs text-muted',
        datetime: item.row.original.created_at,
        locale: 'en',
        timeStyle: 'medium',
        hour12: false,
      }),
    ]),
  },
  {
    accessorKey: 'action',
    header: 'Action Type',
    cell: item => h('span', { class: 'text-sm capitalize whitespace-nowrap' }, item.row.original.action),
  },
  {
    accessorKey: 'summary',
    header: 'Resource',
    cell: item => h('span', { class: 'text-sm whitespace-nowrap' }, item.row.original.summary),
  },
  {
    accessorKey: 'actor_name',
    header: 'Actor',
    cell: item => h('span', { class: 'text-sm whitespace-nowrap' }, item.row.original.actor_name ?? 'System'),
  },
  {
    id: 'context',
    header: 'Actor Context',
    cell: item => h('span', { class: 'text-sm text-muted whitespace-nowrap' }, item.row.original.actor_id ? 'Organization' : '-'),
  },
]
</script>

<template>
  <OrganizationSettingsPage title="Audit log">
    <section class="space-y-4">
      <div class="flex items-center justify-between gap-3">
        <UDropdownMenu :items="auditFilterItems">
          <UButton :icon="ICONS.plus" color="neutral" variant="ghost">Add filter</UButton>
        </UDropdownMenu>
        <USelect
          v-model="auditRange"
          :items="auditRangeItems"
          class="w-36"
          aria-label="Audit log date range"
        />
      </div>

      <UiTable
        :key="`${auditRange}:${auditAction}`"
        v-model:offset="offset"
        :status="status"
        :items="paginatedEvents"
        :columns="columns"
        disable-header
        :pagination="showPagination"
        :total="filteredEvents.length"
        :limit="AUDIT_PAGE_SIZE"
      >
        <template #empty>
          <p class="py-12 text-sm text-muted">No events recorded in this time range.</p>
        </template>
      </UiTable>
    </section>
  </OrganizationSettingsPage>
</template>
