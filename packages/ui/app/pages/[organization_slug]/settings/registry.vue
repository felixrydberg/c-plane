<script setup lang="ts">
import { h } from 'vue'
import { FetchError } from 'ofetch'
import type { TableColumn } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

type RegistryGcRun = {
  started_at: string
  finished_at: string
  bytes_before?: number | null
  bytes_after?: number | null
  result: string
  error?: string | null
}

const store = useStore()
const route = useRoute()
const toast = useToast()
const organizationId = computed(() => store.organization?.id ?? '')
const organizationSlug = computed(() => route.params.organization_slug?.toString() ?? '')
const canManage = computed(() => ['owner', 'admin'].includes(store.organization?.member?.role ?? ''))
const registryUrl = computed(() => organizationId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/registry` as const
  : '')
const regionsUrl = computed(() => organizationId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/regions` as const
  : '')
const { data: registry, refresh: refreshRegistry } = await useCplaneFetch(registryUrl, { default: () => null })
const { data: regions } = await useCplaneFetch(regionsUrl, { default: () => [] })
const garbageCollectionUrl = computed(() => organizationId.value && registry.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/registry/garbage-collection` as const
  : '')
const { data: garbageCollection, status: garbageCollectionStatus, refresh: refreshGarbageCollection } = await useCplaneFetch(garbageCollectionUrl, {
  query: { page: 1, per_page: 10 },
  default: () => null,
})
const regionId = ref('')
const activating = ref(false)
const runningGc = ref(false)
const gcModalOpen = ref(false)
const isActive = computed(() => registry.value?.status === 'active')

watchEffect(() => {
  if (!regionId.value) regionId.value = regions.value[0]?.id ?? ''
})

async function activateRegistry() {
  if (!regionId.value) return
  activating.value = true
  try {
    await cplaneFetch(`/api/organization/${organizationId.value as ':organization_id'}/registry` as const, {
      method: 'PUT',
      body: { region_id: regionId.value },
    })
    await Promise.all([refreshRegistry(), refreshGarbageCollection()])
    toast.add({ title: 'Managed Registry activated', color: 'success' })
  } catch (error) {
    const message = error instanceof FetchError ? error.data?.message : undefined
    toast.add({ title: message || 'Failed to activate Managed Registry', color: 'error' })
  } finally {
    activating.value = false
  }
}

async function runGarbageCollection() {
  runningGc.value = true
  try {
    await cplaneFetch(`/api/organization/${organizationId.value as ':organization_id'}/registry/garbage-collection` as const, { method: 'POST' })
    gcModalOpen.value = false
    await Promise.all([refreshRegistry(), refreshGarbageCollection()])
    toast.add({ title: 'Registry cleanup requested', color: 'success' })
  } catch (error) {
    const message = error instanceof FetchError ? error.data?.message : undefined
    toast.add({ title: message || 'Registry cleanup could not be started', color: 'error' })
  } finally {
    runningGc.value = false
  }
}

function formatTimestamp(value: string) {
  const date = new Date(value)
  const pad = (part: number) => String(part).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}, ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
}

function formatBytes(value: number | null | undefined) {
  if (value === null || value === undefined) return '—'
  if (value === 0) return '0 B'
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB']
  const exponent = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1)
  const amount = value / 1024 ** exponent
  return `${amount.toFixed(exponent === 0 ? 0 : 1)} ${units[exponent]}`
}

function formatResult(value: string) {
  return value ? `${value.charAt(0).toUpperCase()}${value.slice(1)}` : value
}

function reclaimedBytes(run: RegistryGcRun) {
  if (run.bytes_before === null || run.bytes_before === undefined || run.bytes_after === null || run.bytes_after === undefined) return null
  return Math.max(run.bytes_before - run.bytes_after, 0)
}

const UBadge = resolveComponent('UBadge')
const cleanupColumns: TableColumn<RegistryGcRun>[] = [
  {
    id: 'started_at',
    header: 'Started',
    meta: { class: { th: 'text-left', td: 'text-left' }, style: { th: { width: '24%' }, td: { width: '24%' } } },
    cell: ({ row }) => h('span', { class: 'whitespace-nowrap text-sm' }, formatTimestamp(row.original.started_at)),
  },
  {
    id: 'finished_at',
    header: 'Finished',
    meta: { class: { th: 'text-left', td: 'text-left' }, style: { th: { width: '24%' }, td: { width: '24%' } } },
    cell: ({ row }) => h('span', { class: 'whitespace-nowrap text-sm text-muted' }, formatTimestamp(row.original.finished_at)),
  },
  {
    id: 'bytes_before',
    header: 'Size before',
    meta: { class: { th: 'text-left', td: 'text-left' }, style: { th: { width: '14%' }, td: { width: '14%' } } },
    cell: ({ row }) => h('span', { class: 'whitespace-nowrap font-mono text-xs text-muted' }, formatBytes(row.original.bytes_before)),
  },
  {
    id: 'bytes_after',
    header: 'Size after',
    meta: { class: { th: 'text-left', td: 'text-left' }, style: { th: { width: '12%' }, td: { width: '12%' } } },
    cell: ({ row }) => h('span', { class: 'whitespace-nowrap font-mono text-xs text-muted' }, formatBytes(row.original.bytes_after)),
  },
  {
    id: 'reclaimed',
    header: 'Reclaimed',
    meta: { class: { th: 'text-left', td: 'text-left' }, style: { th: { width: '14%' }, td: { width: '14%' } } },
    cell: ({ row }) => h('span', { class: 'whitespace-nowrap font-mono text-xs text-muted' }, formatBytes(reclaimedBytes(row.original))),
  },
  {
    id: 'result',
    header: 'Status',
    meta: { class: { th: 'text-left', td: 'text-left' }, style: { th: { width: '12%' }, td: { width: '12%' } } },
    cell: ({ row }) => h('div', { class: 'flex flex-col items-start gap-1' }, [
      h('span', { class: row.original.result === 'succeeded' ? 'whitespace-nowrap text-sm text-muted' : 'whitespace-nowrap text-sm text-error' }, formatResult(row.original.result)),
      row.original.error ? h('span', { class: 'max-w-56 text-left text-xs text-error' }, row.original.error) : null,
    ]),
  },
]
</script>

<template>
  <div class="flex w-full max-w-5xl flex-col gap-5 mx-auto">
    <header class="border-b border-default/60 pb-5">
      <UiPageEyebrow label="Organization settings" />
      <h1 class="mt-2 text-2xl font-semibold">Registry</h1>
      <p class="mt-1 text-sm text-muted">Manage Registry access and cleanup for {{ organizationSlug }}.</p>
    </header>

    <section v-if="!registry" class="grid gap-6 rounded-lg border border-dashed border-default p-6 lg:grid-cols-[minmax(0,1fr)_320px] lg:items-end">
      <div>
        <UIcon :name="ICONS.registry" class="size-10 text-muted" />
        <h2 class="mt-4 text-lg font-semibold">Activate Managed Registry</h2>
        <p class="mt-2 text-sm text-muted">Choose the permanent storage region for this organization.</p>
      </div>
      <div class="space-y-4">
        <UFormField label="Region" description="This choice is permanent.">
          <USelect v-model="regionId" :items="regions.map(region => ({ label: region.display_name, value: region.id }))" class="w-full" />
        </UFormField>
        <UButton :icon="ICONS.check" color="primary" block :loading="activating" :disabled="!regionId || !canManage" @click="activateRegistry">Activate Registry</UButton>
      </div>
    </section>

    <template v-else>
      <UAlert v-if="!isActive" color="warning" variant="subtle" icon="i-heroicons:exclamation-triangle" title="Registry maintenance in progress" description="Pulls, pushes, and Registry changes are temporarily unavailable." />
      <section class="rounded-xl border border-default/70 bg-default p-5">
        <div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <div class="flex items-center gap-2"><h2 class="text-base font-semibold">Storage cleanup</h2><UBadge size="sm" :color="isActive ? 'success' : 'warning'" variant="subtle">{{ isActive ? 'Active' : 'Maintenance' }}</UBadge></div>
            <p class="mt-1 text-sm text-muted">Removes unused images from the Registry.</p>
          </div>
          <UButton v-if="canManage" size="sm" :icon="ICONS.refresh" color="neutral" :disabled="!isActive || Boolean(garbageCollection?.active_job)" @click="gcModalOpen = true">Run cleanup now</UButton>
        </div>
        <div class="mt-6 border-t border-default/60 pt-5">
          <h3 class="font-semibold">Recent cleanups</h3>
          <UiTable
            class="mt-3"
            :status="garbageCollectionStatus"
            :items="garbageCollection?.gc_runs.data ?? []"
            :columns="cleanupColumns"
            disable-header
          >
            <template #empty>
              <div class="flex flex-col items-center justify-center gap-3 py-12 text-center">
                <UIcon :name="ICONS.revision" class="size-9 text-muted" />
                <p class="text-sm text-muted">No cleanups yet.</p>
                <p class="text-xs text-dimmed">Run cleanup to see storage history and reclaimed space here.</p>
              </div>
            </template>
          </UiTable>
        </div>
      </section>
    </template>

    <UModal v-model:open="gcModalOpen" title="Run Registry cleanup" description="Registry access is temporarily unavailable while cleanup runs.">
      <template #body>
        <div class="flex justify-end gap-3">
          <UButton color="neutral" variant="ghost" @click="gcModalOpen = false">Cancel</UButton>
          <UButton :icon="ICONS.refresh" color="primary" :loading="runningGc" @click="runGarbageCollection">Run cleanup</UButton>
        </div>
      </template>
    </UModal>
  </div>
</template>
