<script setup lang="ts">
import type { Container, ContainerVersion } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'
import { syncEnvironment } from '~/utils/environments'

type ContainerWithProject = Container & {
  _projectName?: string;
  _projectId?: string;
}

const store = useStore()
const route = useRoute()
const router = useRouter()
const projectId = computed(() => route.params.project_id?.toString() || null)
const environmentId = computed(() => route.params.environment_id?.toString() || null)

const orgId = computed(() => store.organization?.id)

const projectName = computed(() =>
  store.projects.find(p => p.id === projectId.value)?.name ?? projectId.value ?? ''
)

const { data: environmentList, refresh: refreshEnvironments } = await useCplaneFetch(
  () => orgId.value && projectId.value ? `/api/organization/${orgId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/environments` as const : '',
  { immediate: computed(() => !!(orgId.value && projectId.value)) },
)

const environmentName = computed(() => {
  if (!environmentId.value) return 'default'
  const list = environmentList.value ?? store.environments
  return list.find(b => b.id === environmentId.value)?.name ?? environmentId.value
})

const fetchedEnvironment = computed(() =>
  environmentList.value?.find(item => item.id === environmentId.value) ?? null
)
const environment = computed(() =>
  store.environment?.id === environmentId.value ? store.environment : fetchedEnvironment.value
)
const hasPendingChanges = computed(() => !!environment.value && environment.value.draft_timeline !== environment.value.deployed_timeline)
const revisionId = computed(() => {
  const revision = route.query.revision
  if (typeof revision === 'string') return revision
  return hasPendingChanges.value ? environment.value?.draft_timeline : environment.value?.deployed_timeline
})
const isViewingDeployed = computed(() => revisionId.value === environment.value?.deployed_timeline)
const isViewingDraft = computed(() => revisionId.value === environment.value?.draft_timeline)

  const fetchUrl = computed(() => {
    const orgId = store.organization?.id
    if (!orgId || !projectId.value || !environmentId.value) return ''
    return `/api/organization/${orgId as ':organization_id'}/containers` as const
  })

const { data, status, refresh: refreshData } = await useLazyCplaneFetch(
  fetchUrl,
  {
    key: 'project-resources',
    query: {
      project_id: projectId,
      environment_id: environmentId,
      timeline_id: revisionId,
    },
    immediate: computed(() => !!(store.organization?.id && projectId.value && environmentId.value)),
  },
)

const fetchReady = computed(() => !!(fetchUrl.value && projectId.value && environmentId.value && environment.value))
const { data: draftContainers, refresh: refreshDraftContainers } = await useCplaneFetch<Container[]>(fetchUrl, {
  key: `environment-draft-containers-${route.params.project_id}-${route.params.environment_id}`,
  query: {
    project_id: projectId,
    environment_id: environmentId,
    timeline_id: computed(() => environment.value?.draft_timeline),
  },
  default: () => [],
  immediate: fetchReady,
})
const { data: deployedContainers, refresh: refreshDeployedContainers } = await useCplaneFetch<Container[]>(fetchUrl, {
  key: `environment-live-containers-${route.params.project_id}-${route.params.environment_id}`,
  query: {
    project_id: projectId,
    environment_id: environmentId,
    timeline_id: computed(() => environment.value?.deployed_timeline),
  },
  default: () => [],
  immediate: fetchReady,
})

const search = ref('')
const refreshing = ref(false)
const changesModalOpen = ref(false)
const deploying = ref(false)
const discarding = ref(false)
const discardModalOpen = ref(false)

const visibleContainers = computed(() => {
  if (revisionId.value === environment.value?.draft_timeline) return draftContainers.value ?? []
  if (revisionId.value === environment.value?.deployed_timeline) return deployedContainers.value ?? []
  return data.value ?? []
})

const containers = computed<ContainerWithProject[]>(() => {
  return visibleContainers.value.map(c => ({
    ...c,
    _projectName: projectName.value,
    _projectId: projectId.value ?? undefined,
  }))
})

const filteredContainers = computed(() => {
  const query = search.value.trim().toLowerCase()
  return query
    ? containers.value.filter(container => `${container.name} ${container.current_version?.image ?? ''}`.toLowerCase().includes(query))
    : containers.value
})

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
    { label: 'CPU', deployed: deployed.cpu ?? 'Default', pending: pending.cpu ?? 'Default' },
    { label: 'Memory', deployed: deployed.memory ?? 'Default', pending: pending.memory ?? 'Default' },
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

const reviewChanges = computed(() => changes.value.map(change => {
  const version = change.container?.current_version
  if (change.fields.length || !version || !['added', 'removed'].includes(change.status)) return change

  const fields = [
    { label: 'Image', value: version.image },
    { label: 'Replicas', value: String(version.replica_count) },
    { label: 'Port', value: String(version.port ?? 'None') },
    { label: 'Access', value: version.public ? 'Public' : 'Private' },
  ].map(({ label, value }) => ({
    label,
    deployed: change.status === 'removed' ? value : '—',
    pending: change.status === 'added' ? value : '—',
  }))
  return { ...change, fields }
}))

const changesSummary = computed(() => {
  const containerCount = changes.value.filter(change => change.container).length
  const counts = (['added', 'changed', 'removed'] as const).flatMap(status => {
    const count = changes.value.filter(change => change.status === status).length
    return count ? [`${count} ${status === 'changed' ? 'updated' : status}`] : []
  })
  return containerCount
    ? [`${containerCount} container${containerCount === 1 ? '' : 's'}`, ...counts].join(' · ')
    : 'Environment release changes'
})

function openChangesModal() {
  if (hasPendingChanges.value) changesModalOpen.value = true
}

function pendingContainerUrl(containerId: string) {
  return {
    path: `/${route.params.organization_slug}/containers/${projectId.value}/${environmentId.value}/${containerId}`,
    query: environment.value?.draft_timeline ? { revision: environment.value.draft_timeline } : undefined,
  }
}

function revisionUrl(revision: string | undefined) {
  return revision ? { query: { ...route.query, revision } } : undefined
}

async function reloadContainers() {
  refreshing.value = true
  try {
    await Promise.all([refreshData(), refreshDraftContainers(), refreshDeployedContainers()])
  } finally {
    refreshing.value = false
  }
}

async function refresh(view: 'draft' | 'deployed') {
  await refreshEnvironments()
  const updated = fetchedEnvironment.value
  if (updated) syncEnvironment(store, updated)
  if (updated && view === 'draft') {
    await navigateTo({ query: { ...route.query, revision: updated.draft_timeline } })
  }
  if (view === 'deployed') {
    await navigateTo({ query: { ...route.query, revision: updated?.deployed_timeline ?? environment.value?.deployed_timeline } })
  }
  await Promise.all([refreshData(), refreshDraftContainers(), refreshDeployedContainers()])
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
    await refreshEnvironments()
    await Promise.all([refreshData(), refreshDraftContainers(), refreshDeployedContainers()])
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
    await refreshEnvironments()
    await router.replace({ query: Object.fromEntries(Object.entries(route.query).filter(([key]) => key !== 'revision')) })
    await Promise.all([refreshData(), refreshDraftContainers(), refreshDeployedContainers()])
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

watch(() => store.refreshKey, () => { void reloadContainers() })
</script>

<template>
  <div class="flex w-full max-w-375 flex-col gap-5 mx-auto">
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

    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div class="min-w-0">
        <UiPageEyebrow label="Compute" />
        <h1 class="mt-2 text-2xl font-semibold">Containers</h1>
        <p class="mt-1 text-sm text-muted">Runtime services in the {{ isViewingDeployed ? 'deployed' : 'draft' }} revision.</p>
      </div>
        <div class="flex shrink-0 items-center gap-2">
          <UButton v-if="isViewingDeployed && hasPendingChanges" :icon="ICONS.pencil" color="neutral" :to="revisionUrl(environment?.draft_timeline)">View draft</UButton>
          <UButton v-if="isViewingDraft" :icon="ICONS.plus" :to="`/${route.params.organization_slug}/containers/${projectId}/${environmentId}/new`">New Container</UButton>
        </div>
    </div>

    <div class="grid overflow-hidden rounded-md border border-dashed border-default bg-transparent sm:grid-cols-3">
      <div class="px-4 py-3"><p class="text-xs text-muted">Environment</p><p class="mt-1 text-sm font-medium">{{ environmentName }}</p></div>
      <div class="px-4 py-3"><p class="text-xs text-muted">Containers</p><p class="mt-1 font-mono text-sm">{{ containers.length }}</p></div>
      <div class="px-4 py-3"><p class="text-xs text-muted">{{ isViewingDraft ? 'Draft' : 'Deployed' }} replicas</p><p class="mt-1 font-mono text-sm">{{ containers.reduce((total, container) => total + (container.current_version?.replica_count ?? 0), 0) }}</p></div>
    </div>

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
        @click="reloadContainers"
      >
        Refresh
      </UButton>
    </div>

    <DeploymentsContainersListing
      v-if="projectId && environmentId && store.organization"
      :containers="filteredContainers"
      :organization-id="store.organization.id"
      :project-id="projectId"
      :environment-id="environmentId"
      :revision-id="revisionId"
      :draft-revision-id="environment?.draft_timeline"
      :can-remove="isViewingDraft"
      :status="status"
      :has-error="status === 'error'"
      @refresh="refresh"
    />

    <UModal v-model:open="changesModalOpen" title="Review changes" :description="changesSummary" :close="false" :ui="{ content: 'max-w-3xl', wrapper: 'w-full' }">
      <template #title>
        <div class="flex w-full flex-wrap items-center justify-between gap-3">
          <span class="text-lg font-semibold">Review changes</span>
          <UButton v-if="isViewingDeployed" :icon="ICONS.pencil" color="neutral" :to="revisionUrl(environment?.draft_timeline)" @click="changesModalOpen = false">View draft</UButton>
          <UButton v-else :icon="ICONS.revision" color="neutral" :to="revisionUrl(environment?.deployed_timeline)" @click="changesModalOpen = false">View live</UButton>
        </div>
      </template>
      <template #body>
        <div class="overflow-x-auto">
          <table class="w-full min-w-[560px] table-fixed text-left text-sm">
            <caption class="sr-only">Container configuration changes from the live release to the pending release</caption>
            <thead class="text-xs font-medium uppercase text-muted">
              <tr class="border-b border-default/60">
                <th scope="col" class="w-[30%] pb-3 pr-4 font-medium">Container</th>
                <th scope="col" class="w-[22%] pb-3 pr-4 font-medium">Setting</th>
                <th scope="col" class="w-[24%] pb-3 pr-4 font-medium">Live</th>
                <th scope="col" class="w-[24%] pb-3 font-medium">Pending</th>
              </tr>
            </thead>
            <tbody v-for="change in reviewChanges" :key="change.id" class="border-b border-default/60 last:border-b-0">
              <tr>
                <th scope="rowgroup" :rowspan="Math.max(change.fields.length, 1) + (change.status === 'removed' ? 1 : 0)" class="py-3 pr-4 align-top font-normal">
                  <div class="flex items-center justify-between gap-3">
                    <span class="min-w-0 break-all font-semibold">{{ change.name }}</span>
                    <UButton v-if="change.container && change.status !== 'removed'" class="shrink-0" size="xs" color="neutral" :icon="ICONS.pencil" :aria-label="`Edit ${change.name}`" :to="pendingContainerUrl(change.container.id)" @click="changesModalOpen = false">Edit</UButton>
                  </div>
                  <p class="mt-1 text-xs" :class="change.status === 'removed' ? 'text-error' : change.status === 'added' ? 'text-success' : change.status === 'changed' ? 'text-warning' : 'text-muted'">
                    {{ change.status === 'changed' ? 'Updated' : change.status === 'release' ? 'Release' : change.status === 'added' ? 'Added' : 'Removed' }}
                  </p>
                </th>
                <template v-if="change.fields[0]">
                  <th scope="row" class="py-3 pr-4 align-top font-normal">{{ change.fields[0].label }}</th>
                  <td class="break-all py-3 pr-4 align-top font-mono text-xs text-muted">{{ change.fields[0].deployed }}</td>
                  <td class="break-all py-3 align-top font-mono text-xs">{{ change.fields[0].pending }}</td>
                </template>
                <td v-else colspan="3" class="py-3 text-sm text-muted">{{ change.summary }}</td>
              </tr>
              <tr v-for="field in change.fields.slice(1)" :key="field.label">
                <th scope="row" class="border-t border-default/40 py-2 pr-4 align-top font-normal">{{ field.label }}</th>
                <td class="break-all border-t border-default/40 py-2 pr-4 align-top font-mono text-xs text-muted">{{ field.deployed }}</td>
                <td class="break-all border-t border-default/40 py-2 align-top font-mono text-xs">{{ field.pending }}</td>
              </tr>
              <tr v-if="change.status === 'removed'">
                <td colspan="3" class="pb-3 text-xs text-muted">This container will stop running when the release is deployed.</td>
              </tr>
            </tbody>
          </table>
        </div>
      </template>
      <template #footer>
        <div class="flex w-full flex-wrap items-center justify-between gap-3">
          <UButton :icon="ICONS.trash" color="error" :loading="discarding" @click="discardModalOpen = true">Discard changes</UButton>
          <div class="ml-auto flex items-center gap-3">
            <UButton color="neutral" variant="ghost" @click="changesModalOpen = false">Close</UButton>
            <UButton :icon="ICONS.check" color="primary" :loading="deploying" @click="deployChanges">Deploy changes</UButton>
          </div>
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
