<script setup lang="ts">
import { h } from 'vue'
import type { TableColumn } from '@nuxt/ui'
import type { Repository } from '@cplane/sdk'
import { FetchError } from 'ofetch'
import { ICONS } from '~/utils/icons'
defineOptions({ name: 'OrganizationRegistryPage' })

const store = useStore()
const route = useRoute()
const config = useRuntimeConfig()
const toast = useToast()
const organizationId = computed(() => store.organization?.id ?? '')
const organizationSlug = computed(() => store.organization?.slug ?? '')
const organizationRole = computed(() => store.organization?.member?.role)
const canManageRegistry = computed(() => organizationRole.value === 'owner' || organizationRole.value === 'admin')
const registryHost = computed(() => config.public.registryHost)
const registryUrl = computed(() => organizationId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/registry` as const
  : '')
const regionsUrl = computed(() => organizationId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/regions` as const
  : '')
const { data: managedRegistry, refresh: refreshRegistry } = await useCplaneFetch(registryUrl, { default: () => null })
const { data: regions } = await useCplaneFetch(regionsUrl, { default: () => [] })
const gcPage = ref(1)
const garbageCollectionUrl = computed(() => organizationId.value && managedRegistry.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/registry/garbage-collection` as const
  : '')
const { data: garbageCollection, refresh: refreshGarbageCollection } = await useCplaneFetch(garbageCollectionUrl, {
  query: computed(() => ({ page: gcPage.value, per_page: 10 })),
  default: () => null,
})
const repositoriesUrl = computed(() => organizationId.value && managedRegistry.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/registry/repositories` as const
  : '')
const { data: repositories, status, refresh: refreshRepositories } = await useCplaneFetch(repositoriesUrl, { default: () => [] })

const selectedRepository = ref<Repository | null>(null)
const deleteModalOpen = ref(false)
const gcModalOpen = ref(false)
const deleting = ref(false)
const activating = ref(false)
const runningGc = ref(false)
const regionId = ref('')
const refreshing = ref(false)
const search = ref('')
const registryIsActive = computed(() => managedRegistry.value?.status === 'active')
const UButton = resolveComponent('UButton')

const filteredRepositories = computed(() => {
  const query = search.value.trim().toLowerCase()
  return query ? repositories.value.filter(repository => repository.name.toLowerCase().includes(query)) : repositories.value
})

async function reloadRepositories() {
  refreshing.value = true
  try {
    await refreshRepositories()
  } finally {
    refreshing.value = false
  }
}

const repositoryColumns: TableColumn<Repository>[] = [
  {
    accessorKey: 'name',
    header: 'Repository',
    cell: ({ row }) => h('span', { class: 'break-all font-mono text-sm' }, row.original.name),
  },
  {
    id: 'reference',
    header: 'Reference',
    meta: { class: { th: 'hidden lg:table-cell', td: 'hidden lg:table-cell' } },
    cell: ({ row }) => h('span', { class: 'break-all font-mono text-xs text-muted' }, `${registryHost.value}/${organizationSlug.value}/${row.original.name}`),
  },
  {
    accessorKey: 'created_at',
    header: 'Created',
    meta: { class: { th: 'hidden sm:table-cell', td: 'hidden sm:table-cell' } },
    cell: ({ row }) => new Date(row.original.created_at).toLocaleDateString(),
  },
  {
    id: 'actions',
    header: '',
    meta: { class: { th: 'text-right', td: 'text-right' } },
    cell: ({ row }) => canManageRegistry.value ? h(UButton, {
      icon: ICONS.trash,
      color: 'error',
      size: 'sm',
      disabled: !registryIsActive.value,
      onClick: () => confirmDelete(row.original),
    }, { default: () => 'Delete' }) : null,
  },
]

function formatTimestamp(value?: string | null) {
  if (!value) return 'Not yet'
  return new Intl.DateTimeFormat('en-GB', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    timeZoneName: 'short',
  }).format(new Date(value))
}

function formatDuration(start: string, end: string) {
  const seconds = Math.max(0, (new Date(end).getTime() - new Date(start).getTime()) / 1000)
  if (seconds < 60) return `${Math.round(seconds)}s`
  const minutes = Math.floor(seconds / 60)
  const rest = Math.round(seconds % 60)
  return rest ? `${minutes}m ${rest}s` : `${minutes}m`
}

function formatBytes(value?: number | null) {
  if (value == null) return '—'
  if (value < 1024) return `${value} B`
  const units = ['KiB', 'MiB', 'GiB', 'TiB']
  let size = value / 1024
  let unit = units[0]
  for (let index = 1; index < units.length && size >= 1024; index++) {
    size /= 1024
    unit = units[index]
  }
  return `${size.toFixed(size >= 100 ? 0 : 1)} ${unit}`
}

interface GcRunRow {
  started: string
  duration: string
  before: string
  after: string
  reclaimed: string
  result: string
  error: string
}

const gcRuns = computed<GcRunRow[]>(() => (garbageCollection.value?.gc_runs.data ?? []).map(run => ({
  started: formatTimestamp(run.started_at),
  duration: formatDuration(run.started_at, run.finished_at),
  before: formatBytes(run.bytes_before),
  after: formatBytes(run.bytes_after),
  reclaimed: run.bytes_before != null && run.bytes_after != null ? formatBytes(run.bytes_before - run.bytes_after) : '—',
  result: run.result === 'succeeded' ? 'Succeeded' : 'Failed',
  error: run.error ?? '',
})))

const gcRunColumns: TableColumn<GcRunRow>[] = [
  { accessorKey: 'started', header: 'Started' },
  { accessorKey: 'duration', header: 'Duration' },
  { accessorKey: 'before', header: 'Space before' },
  { accessorKey: 'after', header: 'Space after' },
  { accessorKey: 'reclaimed', header: 'Reclaimed' },
  {
    accessorKey: 'result',
    header: 'Result',
    cell: ({ row }) => h('div', {
      title: row.original.error,
      class: row.original.result === 'Failed' ? 'text-error' : 'text-success',
    }, row.original.result),
  },
]

watchEffect(() => {
  if (!regionId.value && regions.value.length) regionId.value = regions.value[0]?.id ?? ''
})

let gcPollTimer: ReturnType<typeof setTimeout> | undefined
function pollGarbageCollection() {
  if (gcPollTimer) clearTimeout(gcPollTimer)
  if (!import.meta.client) return
  const shouldPoll = managedRegistry.value?.status === 'maintenance'
    || garbageCollection.value?.active_job?.trigger === 'manual'
  if (!shouldPoll) return
  gcPollTimer = setTimeout(async () => {
    await Promise.all([refreshRegistry(), refreshGarbageCollection(), refreshRepositories()])
    pollGarbageCollection()
  }, 5000)
}

watch(
  () => [managedRegistry.value?.status, garbageCollection.value?.active_job?.id],
  pollGarbageCollection,
  { immediate: true },
)

onBeforeUnmount(() => {
  if (gcPollTimer) clearTimeout(gcPollTimer)
})

async function activateRegistry() {
  if (!organizationId.value || !regionId.value) return
  activating.value = true
  try {
    await cplaneFetch(`/api/organization/${organizationId.value as ':organization_id'}/registry` as const, {
      method: 'PUT',
      body: { region_id: regionId.value },
    })
    await refreshRegistry()
    await refreshGarbageCollection()
    await refreshRepositories()
    toast.add({ title: 'Managed Registry activated', color: 'success' })
  } catch (error) {
    const message = error instanceof FetchError ? error.data?.message : undefined
    toast.add({ title: message || 'Failed to activate Managed Registry', color: 'error' })
  } finally {
    activating.value = false
  }
}

async function runGarbageCollection() {
  if (!organizationId.value) return
  runningGc.value = true
  try {
    await cplaneFetch(`/api/organization/${organizationId.value as ':organization_id'}/registry/garbage-collection` as const, { method: 'POST' })
    gcModalOpen.value = false
    await Promise.all([refreshRegistry(), refreshGarbageCollection()])
    pollGarbageCollection()
    toast.add({ title: 'Registry cleanup queued', description: 'Registry access will pause while cleanup runs.', color: 'success' })
  } catch (error) {
    const message = error instanceof FetchError ? error.data?.message : undefined
    toast.add({ title: message || 'Failed to queue Registry cleanup', color: 'error' })
  } finally {
    runningGc.value = false
  }
}

function confirmDelete(repository: Repository) {
  selectedRepository.value = repository
  deleteModalOpen.value = true
}

async function deleteRepository() {
  if (!selectedRepository.value || !organizationId.value) return
  deleting.value = true
  try {
    await cplaneFetch(`/api/organization/${organizationId.value as ':organization_id'}/registry/repositories/${selectedRepository.value.id as ':repository_id'}` as const, { method: 'DELETE' })
    toast.add({ title: 'Repository and images deleted', description: 'Storage is reclaimed by the next Registry cleanup.', color: 'success' })
    deleteModalOpen.value = false
    selectedRepository.value = null
    await refreshRepositories()
  } catch (error) {
    const message = error instanceof FetchError ? error.data?.message : undefined
    toast.add({ title: message || 'Failed to delete repository and images', color: 'error' })
  } finally {
    deleting.value = false
  }
}
</script>

<template>
  <div class="flex w-full max-w-375 flex-col gap-5 mx-auto">
    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <UiPageEyebrow label="Storage &amp; Databases" />
        <h1 class="text-2xl font-semibold">S2 - Registry</h1>
        <p class="text-muted text-sm mt-1">Private repositories for {{ organizationSlug }}.</p>
      </div>
      <div v-if="managedRegistry" class="flex flex-wrap justify-end gap-2">
        <UButton :icon="ICONS.authentication" color="neutral" :to="`/${organizationSlug}/registry/access-tokens`">Manage access tokens</UButton>
        <UButton :icon="ICONS.plus" color="primary" :to="`/${route.params.organization_slug}/registry/new`" :disabled="!registryIsActive">New repository</UButton>
      </div>
    </div>

    <section v-if="!managedRegistry" class="grid gap-6 rounded-lg border border-dashed border-default p-6 lg:grid-cols-[minmax(0,1fr)_320px] lg:items-end">
      <div>
        <UIcon :name="ICONS.registry" class="size-10 text-muted" />
        <h2 class="mt-4 text-lg font-semibold">Activate Managed Registry</h2>
        <p class="mt-2 max-w-2xl text-sm text-muted">Choose the permanent storage region for this organization. Activation creates one private foundation bucket and enables daily cleanup at 03:00 UTC.</p>
      </div>
      <div class="space-y-4">
        <UFormField label="Region" description="This choice is permanent.">
          <USelect v-model="regionId" :items="regions.map(region => ({ label: region.display_name, value: region.id }))" placeholder="Select a region" class="w-full" :disabled="activating" />
        </UFormField>
        <UButton :icon="ICONS.check" color="primary" block :loading="activating" :disabled="!regionId || !canManageRegistry" @click="activateRegistry">Activate Registry</UButton>
      </div>
    </section>

    <template v-else>
      <UAlert
        v-if="!registryIsActive"
        color="warning"
        variant="subtle"
        icon="i-heroicons:exclamation-triangle"
        title="Registry maintenance in progress"
        description="Pulls, pushes, and Registry changes are temporarily unavailable while garbage collection runs."
      />

      <div class="flex items-center gap-2">
        <UInput v-model="search" icon="i-heroicons:magnifying-glass" placeholder="Search repositories..." aria-label="Search repositories" class="min-w-0 flex-1" />
        <UButton :icon="ICONS.refresh" color="neutral" :loading="refreshing" @click="reloadRepositories">Refresh</UButton>
      </div>

      <UiTable :status="status" :items="filteredRepositories" :columns="repositoryColumns" disable-header>
        <template #empty>
          <div class="flex flex-col items-center justify-center gap-3 py-14 text-center">
            <UIcon :name="ICONS.registry" class="size-10 text-muted" />
            <p class="text-muted">{{ search ? 'No matching repositories.' : 'No repositories yet.' }}</p>
            <p v-if="!search" class="text-dimmed text-sm">Create your first repository before pushing an image.</p>
          </div>
        </template>
      </UiTable>

      <section class="rounded-lg border border-default p-5">
        <div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <div class="flex items-center gap-2">
              <h2 class="text-lg font-semibold">Garbage collection</h2>
              <UBadge :color="registryIsActive ? 'success' : 'warning'" variant="subtle">{{ registryIsActive ? 'Active' : 'Maintenance' }}</UBadge>
            </div>
            <p class="mt-1 text-sm text-muted">Deleted images continue using storage until cleanup finishes.</p>
          </div>
          <UButton
            v-if="canManageRegistry"
            :icon="ICONS.refresh"
            color="neutral"
            :disabled="!registryIsActive || Boolean(garbageCollection?.active_job && garbageCollection.active_job.trigger === 'manual')"
            @click="gcModalOpen = true"
          >Run cleanup now</UButton>
        </div>

        <div class="mt-6 border-t border-default/60 pt-5">
          <h3 class="font-semibold">Recent cleanups</h3>
          <p class="mt-1 text-sm text-muted">Latest garbage-collection runs.</p>
          <UTable
            v-if="gcRuns.length"
            :data="gcRuns"
            :columns="gcRunColumns"
            class="mt-4"
            :ui="{
              thead: '[&>tr>th]:py-1 [&>tr>th]:px-4 [&>tr>th]:text-sm',
              td: 'py-2 px-4 bg-elevated/50 border-y border-default first:rounded-l-lg last:rounded-r-lg first:border-l last:border-r text-sm',
            }"
          />
          <UPagination
            v-if="garbageCollection?.gc_runs.pagination.total_pages > 1"
            v-model:page="gcPage"
            :total="garbageCollection.gc_runs.pagination.total"
            :items-per-page="garbageCollection.gc_runs.pagination.per_page"
            class="mt-4 mx-auto flex justify-center"
          />
          <p v-if="!gcRuns.length" class="mt-4 text-sm text-muted">No cleanups yet.</p>
        </div>
      </section>
    </template>

    <UModal v-model:open="gcModalOpen" title="Run Registry cleanup" description="Registry access will pause while garbage collection removes unreferenced image data.">
      <template #body>
        <div class="space-y-4">
          <p class="text-sm">Existing pulls and pushes may fail until cleanup completes. The Registry returns to active status automatically.</p>
          <div class="flex justify-end gap-3 pt-2">
            <UButton color="neutral" variant="ghost" :disabled="runningGc" @click="gcModalOpen = false">Cancel</UButton>
            <UButton :icon="ICONS.refresh" color="primary" :loading="runningGc" @click="runGarbageCollection">Run cleanup</UButton>
          </div>
        </div>
      </template>
    </UModal>

    <UModal v-model:open="deleteModalOpen" title="Delete repository" description="This deletes the repository images and access permissions. Storage is reclaimed by the next cleanup.">
      <template #body>
        <div class="space-y-4">
          <p class="text-sm">Are you sure you want to delete <strong>{{ selectedRepository?.name }}</strong>?</p>
          <div class="flex justify-end gap-3 pt-2">
            <UButton color="neutral" variant="ghost" :disabled="deleting" @click="deleteModalOpen = false">Cancel</UButton>
            <UButton :icon="ICONS.trash" color="error" :loading="deleting" @click="deleteRepository">Delete</UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>
