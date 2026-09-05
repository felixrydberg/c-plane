<script setup lang="ts">
import { h } from 'vue'
import type { Container, ContainerVersion } from '@cplane/sdk'
import type { TableColumn } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'
import { syncEnvironment } from '~/utils/environments'

type ChangeField = {
  label: string
  deployed: string
  pending: string
}

type ContainerChange = {
  id: string
  name: string
  status: 'added' | 'changed' | 'removed' | 'release'
  summary: string
  fields: ChangeField[]
  container: Container | null
}

const store = useStore()
const route = useRoute()
const toast = useToast()

const organizationSlug = computed(() => route.params.organization_slug?.toString() || '')
const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() || '')
const environmentId = computed(() => route.params.environment_id?.toString() || '')

const environmentsUrl = computed(() => orgId.value && projectId.value
  ? `/api/organization/${orgId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/environments` as const
  : '')
const { data: environmentList, refresh: refreshEnvironments } = await useCplaneFetch(environmentsUrl, {
  immediate: computed(() => !!environmentsUrl.value),
})
const fetchedEnvironment = computed(() => environmentList.value?.find(item => item.id === environmentId.value) ?? null)
const environment = computed(() => store.environment?.id === environmentId.value ? store.environment : fetchedEnvironment.value)
const hasPendingChanges = computed(() => !!environment.value && environment.value.draft_timeline !== environment.value.deployed_timeline)

const containersUrl = computed(() => orgId.value ? `/api/organization/${orgId.value as ':organization_id'}/containers` as const : '')
const fetchReady = computed(() => !!(containersUrl.value && projectId.value && environmentId.value && environment.value))
const { data: draftContainers, refresh: refreshDraftContainers } = await useCplaneFetch<Container[]>(containersUrl, {
  key: `environment-draft-containers-${route.params.project_id}-${route.params.environment_id}`,
  query: {
    project_id: projectId,
    environment_id: environmentId,
    timeline_id: computed(() => environment.value?.draft_timeline),
  },
  default: () => [],
  immediate: fetchReady,
})
const { data: deployedContainers, status: deployedStatus, refresh: refreshDeployedContainers } = await useCplaneFetch<Container[]>(containersUrl, {
  key: `environment-live-containers-${route.params.project_id}-${route.params.environment_id}`,
  query: {
    project_id: projectId,
    environment_id: environmentId,
    timeline_id: computed(() => environment.value?.deployed_timeline),
  },
  default: () => [],
  immediate: fetchReady,
})

const expandedChangeId = ref<string | null>(null)
const changesModalOpen = ref(false)
const deploying = ref(false)
const discarding = ref(false)
const discardModalOpen = ref(false)
const refreshing = ref(false)
const search = ref('')

const filteredDeployedContainers = computed(() => {
  const query = search.value.trim().toLowerCase()
  const containers = deployedContainers.value ?? []
  if (!query) return containers

  return containers.filter((container) => {
    const version = container.current_version
    return [container.name, version?.image, version?.resolved_image]
      .filter(Boolean)
      .join(' ')
      .toLowerCase()
      .includes(query)
  })
})

function stableValue(value: unknown): string {
  if (value === null || value === undefined) return ''
  if (Array.isArray(value)) return `[${value.map(stableValue).join(',')}]`
  if (typeof value !== 'object') return String(value)
  return `{${Object.entries(value as Record<string, unknown>)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([key, entry]) => `${key}:${stableValue(entry)}`)
    .join(',')}}`
}

function objectCount(value: unknown): number {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? Object.keys(value as Record<string, unknown>).length
    : 0
}

function healthCheckPath(version: ContainerVersion | null | undefined): string {
  const healthCheck = version?.health_check
  if (!healthCheck || typeof healthCheck !== 'object' || !('path' in healthCheck)) return 'None'
  return String((healthCheck as { path?: unknown }).path ?? 'None')
}

function changeFields(deployed: ContainerVersion | null | undefined, pending: ContainerVersion | null | undefined): ChangeField[] {
  if (!deployed || !pending) return []

  const values: Array<ChangeField & { deployedKey?: string, pendingKey?: string }> = [
    { label: 'Image', deployed: deployed.image, pending: pending.image },
    { label: 'Resolved image', deployed: deployed.resolved_image, pending: pending.resolved_image },
    { label: 'Replicas', deployed: String(deployed.replica_count), pending: String(pending.replica_count) },
    { label: 'Port', deployed: deployed.port === null || deployed.port === undefined ? 'None' : String(deployed.port), pending: pending.port === null || pending.port === undefined ? 'None' : String(pending.port) },
    { label: 'Access', deployed: deployed.public ? 'Public' : 'Private', pending: pending.public ? 'Public' : 'Private' },
    { label: 'Health check', deployed: healthCheckPath(deployed), pending: healthCheckPath(pending) },
    { label: 'Environment variables', deployed: `${objectCount(deployed.env)} configured`, pending: `${objectCount(pending.env)} configured`, deployedKey: stableValue(deployed.env), pendingKey: stableValue(pending.env) },
    { label: 'Resources', deployed: stableValue(deployed.resources) || 'Default', pending: stableValue(pending.resources) || 'Default' },
    { label: 'External registry', deployed: deployed.external_registry_id ?? 'None', pending: pending.external_registry_id ?? 'None' },
  ]

  return values
    .filter(value => (value.deployedKey ?? value.deployed) !== (value.pendingKey ?? value.pending))
    .map(({ label, deployed: deployedValue, pending: pendingValue }) => ({ label, deployed: deployedValue, pending: pendingValue }))
}

const changes = computed<ContainerChange[]>(() => {
  const draftById = new Map((draftContainers.value ?? []).map(container => [container.id, container]))
  const deployedById = new Map((deployedContainers.value ?? []).map(container => [container.id, container]))
  const result: ContainerChange[] = []

  for (const container of draftContainers.value ?? []) {
    const deployed = deployedById.get(container.id)
    if (!deployed) {
      result.push({ id: container.id, name: container.name, status: 'added', summary: 'New container', fields: [], container })
      continue
    }

    const fields = changeFields(deployed.current_version, container.current_version)
    if (fields.length > 0 || deployed.current_version?.id !== container.current_version?.id) {
      const differenceCount = fields.length || 1
      result.push({
        id: container.id,
        name: container.name,
        status: 'changed',
        summary: `${differenceCount} configuration change${differenceCount === 1 ? '' : 's'}`,
        fields,
        container,
      })
    }
  }

  for (const container of deployedContainers.value ?? []) {
    if (draftById.has(container.id)) continue
    result.push({ id: container.id, name: container.name, status: 'removed', summary: 'Container removed', fields: [], container })
  }

  if (hasPendingChanges.value && result.length === 0) {
    result.push({
      id: 'release-change',
      name: 'Environment release',
      status: 'release',
      summary: 'A non-container resource version changed',
      fields: [],
      container: null,
    })
  }

  return result
})

watch(changes, (next) => {
  if (!next.length) expandedChangeId.value = null
  else if (!expandedChangeId.value || !next.some(change => change.id === expandedChangeId.value)) expandedChangeId.value = next[0]?.id ?? null
}, { immediate: true })

function openChangesModal() {
  if (hasPendingChanges.value) changesModalOpen.value = true
}

function containerUrl(containerId: string, timeline?: string) {
  const path = `/${organizationSlug.value}/compute/containers/${projectId.value}/${environmentId.value}/${containerId}`
  return timeline ? `${path}?revision=${encodeURIComponent(timeline)}` : path
}

const NuxtLink = resolveComponent('NuxtLink')
const NuxtTime = resolveComponent('NuxtTime')
const deployedColumns: TableColumn<Container>[] = [
  {
    accessorKey: 'name',
    header: 'Service',
    cell: ({ row }) => h(NuxtLink, {
      to: containerUrl(row.original.id, environment.value?.deployed_timeline),
      class: 'flex min-w-0 items-center gap-2',
    }, () => [
      h('span', { class: 'truncate font-medium text-primary group-hover:underline group-hover:underline-offset-4' }, row.original.name),
      row.original.current_version?.public
        ? h('span', { class: 'shrink-0 rounded-full bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-700 dark:text-emerald-400' }, 'Public')
        : null,
    ]),
  },
  {
    id: 'image',
    header: 'Image',
    cell: ({ row }) => h('code', { class: 'block max-w-56 truncate text-xs text-muted' }, row.original.current_version?.image ?? 'No version'),
  },
  {
    id: 'replicas',
    header: 'Replicas',
    cell: ({ row }) => row.original.current_version?.replica_count ?? 0,
  },
  {
    id: 'port',
    header: 'Port',
    cell: ({ row }) => row.original.current_version?.port ?? '—',
  },
  {
    id: 'access',
    header: 'Access',
    cell: ({ row }) => row.original.current_version?.public ? 'Public' : 'Private',
  },
  {
    accessorKey: 'updated_at',
    header: 'Updated',
    cell: ({ row }) => h(NuxtTime, { datetime: row.original.updated_at, relative: true, class: 'whitespace-nowrap text-xs text-muted' }),
  },
]

async function refreshAll() {
  refreshing.value = true
  try {
    await Promise.all([refreshEnvironments(), refreshDraftContainers(), refreshDeployedContainers()])
    const updated = fetchedEnvironment.value
    if (updated) syncEnvironment(store, updated)
  } finally {
    refreshing.value = false
  }
}

async function deployChanges() {
  if (!orgId.value || !projectId.value || !environment.value || !hasPendingChanges.value) return
  deploying.value = true
  try {
    const updated = await cplaneFetch(
      `/api/organization/${orgId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/environments/${environment.value.id as ':environment_id'}` as const,
      { method: 'PATCH', body: { deployed_timeline_id: environment.value.draft_timeline } },
    )
    syncEnvironment(store, updated)
    await Promise.all([refreshEnvironments(), refreshDraftContainers(), refreshDeployedContainers()])
    store.refreshKey++
    changesModalOpen.value = false
    toast.add({ title: `Changes deployed to ${updated.name}`, color: 'success' })
  } catch {
    toast.add({ title: 'Failed to deploy changes', color: 'error' })
  } finally {
    deploying.value = false
  }
}

async function discardChanges() {
  if (!orgId.value || !projectId.value || !environment.value || !hasPendingChanges.value) return
  discarding.value = true
  try {
    const updated = await cplaneFetch(
      `/api/organization/${orgId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/environments/${environment.value.id as ':environment_id'}` as const,
      { method: 'PATCH', body: { draft_timeline_id: environment.value.deployed_timeline } },
    )
    syncEnvironment(store, updated)
    await Promise.all([refreshEnvironments(), refreshDraftContainers(), refreshDeployedContainers()])
    discardModalOpen.value = false
    changesModalOpen.value = false
    store.refreshKey++
    toast.add({ title: 'Pending changes discarded', color: 'success' })
  } catch {
    toast.add({ title: 'Failed to discard changes', color: 'error' })
  } finally {
    discarding.value = false
  }
}

watch(() => store.refreshKey, () => { void refreshAll() })
</script>

<template>
  <div class="mx-auto flex w-full max-w-375 flex-col gap-5">
    <UAlert
      v-if="hasPendingChanges"
      color="warning"
      variant="subtle"
      icon="i-heroicons:exclamation-triangle"
      :title="`${changes.length} pending change${changes.length === 1 ? '' : 's'} in ${environment?.name}`"
      description="Review what will go live before deploying this environment."
      orientation="horizontal"
      class="cursor-pointer border border-warning/40 bg-warning/15 text-warning-800 transition-colors hover:bg-warning/20 dark:border-warning-400/40 dark:bg-warning-950/40 dark:text-warning-200 dark:hover:bg-warning-950/60"
      @click="openChangesModal"
    >
      <template #actions>
        <UButton :icon="ICONS.revision" color="primary" size="sm" @click.stop="openChangesModal">Review changes</UButton>
      </template>
    </UAlert>

    <header class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div class="min-w-0">
        <div class="flex flex-wrap items-center gap-2">
          <UiPageEyebrow :label="environment?.name || 'Environment'" />
          <UBadge v-if="environment?.is_preview" color="primary" variant="soft" size="sm">Preview</UBadge>
        </div>
        <h1 class="mt-2 text-2xl font-semibold">Containers</h1>
        <p class="mt-1 text-sm text-muted">Runtime services in this environment.</p>
      </div>
      <UButton :icon="ICONS.plus" color="primary" :to="`/${organizationSlug}/compute/containers/${projectId}/${environmentId}/new`">Add container</UButton>
    </header>

    <div class="flex items-center gap-2">
      <UInput
        v-model="search"
        icon="i-heroicons:magnifying-glass"
        placeholder="Search containers..."
        aria-label="Search containers"
        class="min-w-0 flex-1"
      />
      <UButton
        :icon="ICONS.refresh"
        color="neutral"
        :loading="refreshing"
        aria-label="Refresh containers"
        @click="refreshAll"
      >
        Refresh
      </UButton>
    </div>

    <section>
      <div class="mb-3 flex flex-col gap-1 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h2 class="text-base font-semibold">Currently deployed</h2>
          <p class="mt-1 text-sm text-muted">Services running in {{ environment?.name || 'this environment' }}.</p>
        </div>
        <p class="font-mono text-xs text-muted">{{ deployedContainers.length }} container{{ deployedContainers.length === 1 ? '' : 's' }}</p>
      </div>

      <UiTable
        :status="deployedStatus"
        :items="filteredDeployedContainers"
        :columns="deployedColumns"
        disable-header
        selectable
        @select="row => navigateTo(containerUrl(row.original.id, environment?.deployed_timeline))"
      >
        <template #empty>
          <div class="flex flex-col items-center justify-center gap-3 py-14 text-center">
            <UIcon :name="ICONS.containers" class="size-10 text-muted" />
            <p class="text-muted">{{ search ? 'No matching containers.' : 'No containers yet.' }}</p>
            <p v-if="!search" class="text-sm text-dimmed">Add your first container to run a service in this environment.</p>
          </div>
        </template>
      </UiTable>
    </section>

    <UModal v-model:open="changesModalOpen" title="Review pending changes" description="Only configuration that differs from the live release is shown." :close="false" :ui="{ content: 'max-w-3xl' }">
      <template #body>
        <div class="space-y-3">
          <div class="flex items-center justify-between gap-3">
            <p class="text-sm font-medium">{{ changes.length }} change{{ changes.length === 1 ? '' : 's' }} ready to review</p>
            <p class="font-mono text-xs text-muted">Live → Pending</p>
          </div>

          <div v-if="changes.length" class="overflow-hidden rounded-lg border border-default/60">
            <article v-for="(change, index) in changes" :key="change.id" :class="index ? 'border-t border-default/60' : ''">
              <button type="button" class="flex w-full items-center gap-3 px-3 py-3 text-left transition-colors hover:bg-elevated/30" :aria-expanded="expandedChangeId === change.id" @click="expandedChangeId = expandedChangeId === change.id ? null : change.id">
                <span class="flex size-8 shrink-0 items-center justify-center rounded-md border border-default/60 bg-elevated/40">
                  <UIcon :name="change.status === 'release' ? ICONS.revision : ICONS.containers" class="size-4 text-muted" />
                </span>
                <span class="min-w-0 flex-1">
                  <span class="flex flex-wrap items-center gap-2">
                    <span class="truncate text-sm font-semibold">{{ change.name }}</span>
                    <UBadge :color="change.status === 'removed' ? 'error' : change.status === 'added' ? 'success' : 'warning'" variant="soft" size="sm">
                      {{ change.status === 'changed' ? 'Updated' : change.status === 'release' ? 'Release' : change.status }}
                    </UBadge>
                  </span>
                  <span class="mt-0.5 block text-xs text-muted">{{ change.summary }}</span>
                </span>
                <UIcon :name="expandedChangeId === change.id ? 'i-heroicons:chevron-up' : 'i-heroicons:chevron-down'" class="size-4 shrink-0 text-muted" />
              </button>

              <div v-if="expandedChangeId === change.id" class="border-t border-default/60 bg-elevated/15 px-3 py-3 sm:pl-14">
                <div v-if="change.fields.length" class="overflow-hidden rounded-md border border-default/60">
                  <div class="hidden grid-cols-[150px_minmax(0,1fr)_20px_minmax(0,1fr)] gap-2 border-b border-default/60 bg-elevated/30 px-3 py-2 text-[10px] font-medium uppercase tracking-wide text-muted sm:grid">
                    <span>Configuration</span><span>Live</span><span /><span>Pending</span>
                  </div>
                  <div v-for="field in change.fields" :key="field.label" class="grid gap-1.5 border-b border-default/60 px-3 py-2 text-sm last:border-b-0 sm:grid-cols-[150px_minmax(0,1fr)_20px_minmax(0,1fr)] sm:items-center sm:gap-2">
                    <span class="text-xs font-medium text-muted">{{ field.label }}</span>
                    <code class="min-w-0 break-all rounded bg-default/80 px-2 py-1 font-mono text-[11px] text-muted">{{ field.deployed }}</code>
                    <UIcon name="i-heroicons:arrow-right" class="hidden size-3.5 text-muted sm:block" />
                    <code class="min-w-0 break-all rounded bg-primary/10 px-2 py-1 font-mono text-[11px] text-highlighted">{{ field.pending }}</code>
                  </div>
                </div>

                <div v-else-if="change.status === 'added' && change.container?.current_version" class="grid gap-3 text-sm sm:grid-cols-4">
                  <div><p class="text-xs text-muted">Image</p><p class="mt-1 truncate font-mono text-xs">{{ change.container.current_version.image }}</p></div>
                  <div><p class="text-xs text-muted">Replicas</p><p class="mt-1">{{ change.container.current_version.replica_count }}</p></div>
                  <div><p class="text-xs text-muted">Port</p><p class="mt-1">{{ change.container.current_version.port ?? 'None' }}</p></div>
                  <div><p class="text-xs text-muted">Access</p><p class="mt-1">{{ change.container.current_version.public ? 'Public' : 'Private' }}</p></div>
                </div>

                <p v-else-if="change.status === 'removed'" class="text-sm text-muted">This container will stop running when the release is deployed.</p>
                <p v-else class="text-sm text-muted">The environment timeline changed, but there is no container configuration difference to display.</p>

                <UButton v-if="change.container && change.status !== 'removed'" class="mt-3" color="neutral" variant="ghost" :icon="ICONS.pencil" :to="containerUrl(change.container.id)" @click="changesModalOpen = false">Edit container</UButton>
              </div>
            </article>
          </div>
        </div>
      </template>
      <template #footer>
        <div class="flex w-full justify-end gap-3">
          <UButton color="neutral" variant="ghost" @click="changesModalOpen = false">Close</UButton>
          <UButton :icon="ICONS.trash" color="error" :loading="discarding" @click="discardModalOpen = true">Discard changes</UButton>
          <UButton :icon="ICONS.check" color="primary" :loading="deploying" @click="deployChanges">Deploy changes</UButton>
        </div>
      </template>
    </UModal>

    <UModal v-model:open="discardModalOpen" title="Discard pending changes?" :close="false" :ui="{ content: 'max-w-md' }">
      <template #body>
        <p class="text-sm text-muted">This resets {{ environment?.name }} to the currently deployed release. The pending container changes will be removed.</p>
        <div class="mt-5 flex justify-end gap-3">
          <UButton color="neutral" variant="ghost" :disabled="discarding" @click="discardModalOpen = false">Cancel</UButton>
          <UButton :icon="ICONS.trash" color="error" :loading="discarding" @click="discardChanges">Discard changes</UButton>
        </div>
      </template>
    </UModal>
  </div>
</template>
