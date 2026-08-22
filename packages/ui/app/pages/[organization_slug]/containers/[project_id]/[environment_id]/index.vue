<script setup lang="ts">
import type { Container } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'
import { syncEnvironment } from '~/utils/environments'

type ContainerWithProject = Container & {
  _projectName?: string;
  _projectId?: string;
}

const store = useStore()
const route = useRoute()
const projectId = computed(() => route.params.project_id?.toString() || null)
const environmentId = computed(() => route.params.environment_id?.toString() || null)

const orgId = computed(() => store.organization?.id)

const projectName = computed(() =>
  store.projects.find(p => p.id === projectId.value)?.name ?? projectId.value ?? ''
)

const { data: environmentList, refresh: refreshEnvironments } = await useFetch(
  () => orgId.value && projectId.value ? `/api/cplane/organization/${orgId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/environments` as const : '',
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
const revisionId = computed(() => {
  const revision = route.query.revision
  return typeof revision === 'string' ? revision : environment.value?.deployed_timeline
})
const isViewingDeployed = computed(() => revisionId.value === environment.value?.deployed_timeline)
const isViewingDraft = computed(() => revisionId.value === environment.value?.draft_timeline)

  const fetchUrl = computed(() => {
    const orgId = store.organization?.id
    if (!orgId || !projectId.value || !environmentId.value) return ''
    return `/api/cplane/organization/${orgId as ':organization_id'}/containers` as const
  })

const { data, status, refresh: refreshData } = await useLazyFetch(
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

const search = ref('')
const refreshing = ref(false)

const containers = computed<ContainerWithProject[]>(() => {
  return (data.value ?? []).map(c => ({
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

async function reloadContainers() {
  refreshing.value = true
  try {
    await refreshData()
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
    const { revision, ...query } = route.query
    await navigateTo({ query })
  }
  await refreshData()
}

watch(() => store.refreshKey, () => { refreshData() })
</script>

<template>
  <div class="flex w-full max-w-375 flex-col gap-5 mx-auto">
    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div class="min-w-0">
        <UiPageEyebrow label="Compute" />
        <h1 class="text-2xl font-semibold">C1 - Containers</h1>
        <p class="mt-1 text-sm text-muted">Runtime services in the {{ isViewingDeployed ? 'deployed' : 'draft' }} revision.</p>
      </div>
      <UButton class="shrink-0" :icon="ICONS.plus" :to="`/${route.params.organization_slug}/containers/${projectId}/${environmentId}/new`">New Container</UButton>
    </div>

    <div class="grid overflow-hidden rounded-md border border-dashed border-default bg-transparent sm:grid-cols-3">
      <div class="px-4 py-3"><p class="text-xs text-muted">Environment</p><p class="mt-1 text-sm font-medium">{{ environmentName }}</p></div>
      <div class="px-4 py-3"><p class="text-xs text-muted">C1 - Containers</p><p class="mt-1 font-mono text-sm">{{ containers.length }}</p></div>
      <div class="px-4 py-3"><p class="text-xs text-muted">{{ isViewingDeployed ? 'Deployed' : 'Draft' }} replicas</p><p class="mt-1 font-mono text-sm">{{ containers.reduce((total, container) => total + (container.current_version?.replica_count ?? 0), 0) }}</p></div>
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
        variant="ghost"
        color="neutral"
        :loading="refreshing"
        aria-label="Reload containers"
        @click="reloadContainers"
      />
    </div>

    <DeploymentsContainersListing
      v-if="projectId && environmentId && store.organization"
      :containers="filteredContainers"
      :organization-id="store.organization.id"
      :project-id="projectId"
      :environment-id="environmentId"
      :revision-id="revisionId"
      :draft-revision-id="environment?.draft_timeline"
      :can-remove="isViewingDraft || isViewingDeployed"
      :status="status"
      :has-error="status === 'error'"
      @refresh="refresh"
    />

  </div>
</template>
