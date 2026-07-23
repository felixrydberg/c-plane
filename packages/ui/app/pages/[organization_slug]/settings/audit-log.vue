<script setup lang="ts">
import { h } from 'vue'
import type { TableColumn } from '@nuxt/ui'

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
const endpoint = `/api/cplane/organization/${store.organization.id as ':organization_id'}/events` as const
const { data: events, status } = await useFetch<AuditEvent[]>(endpoint, {
  query: { limit: 50 },
  default: () => [],
})

const columns: TableColumn<AuditEvent>[] = [
  { accessorKey: 'summary', header: 'Event' },
  {
    accessorKey: 'action',
    header: 'Action',
    cell: (item) => h('span', { class: 'font-mono text-xs capitalize' }, item.row.original.action.replaceAll('_', ' ')),
  },
  {
    accessorKey: 'actor_id',
    header: 'Actor',
    cell: (item) => h('span', { class: 'text-xs text-muted' }, item.row.original.actor_name ?? (item.row.original.actor_id ? 'Unknown actor' : 'System')),
  },
  {
    accessorKey: 'created_at',
    header: 'When',
    cell: (item) => h(NuxtTime, {
      class: 'text-xs text-muted',
      datetime: item.row.original.created_at,
      locale: 'en',
      dateStyle: 'short',
      timeStyle: 'short',
      hour12: false,
    }),
  },
]
</script>

<template>
  <OrganizationSettingsPage title="Audit log">

    <section class="border-b border-dashed border-default pb-10">
      <div>
        <h2 class="text-xl font-normal tracking-[-0.02em]">Recent activity</h2>
        <p class="mt-2 text-sm text-muted">Recent events across this organization.</p>
      </div>

      <div class="mt-8">
        <UiTable :status="status" :items="events" :columns="columns" disable-header>
          <template #empty>
            <p class="py-12 text-sm text-muted">No events recorded yet.</p>
          </template>
        </UiTable>
      </div>
    </section>
  </OrganizationSettingsPage>
</template>
