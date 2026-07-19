<script setup lang="ts">
import type { ContainerRow, ContainerVersionRow } from '@cplane/ui-shared/types';
import { ICONS } from '~/utils/icons'

interface Container extends ContainerRow {
  current_version: ContainerVersionRow | null;
  project_id: string | null;
}

interface ContainerWithProject extends Container {
  _projectName?: string;
  _projectId?: string;
}

const store = useStore()
const route = useRoute()
const projectId = computed(() => route.params.project_id?.toString() || null)
const environmentId = computed(() => route.params.environment_id?.toString() || null)

type EnvironmentItem = { id: string; name: string; timeline: string; is_default: boolean }

const orgId = computed(() => store.organization?.id)

const projectName = computed(() =>
  store.projects.find(p => p.id === projectId.value)?.name ?? projectId.value ?? ''
)

const { data: environmentList } = await useFetch<EnvironmentItem[]>(
  () => orgId.value && projectId.value ? `/api/backend/organization/${orgId.value}/projects/${projectId.value}/environments` : '',
  { immediate: computed(() => !!(orgId.value && projectId.value)) },
)

const environmentName = computed(() => {
  if (!environmentId.value) return 'default'
  const list = environmentList.value ?? store.environments
  return list.find(b => b.id === environmentId.value)?.name ?? environmentId.value
})

const fetchUrl = computed(() => {
  const orgId = store.organization?.id
  if (!orgId || !projectId.value || !environmentId.value) return ''
  return `/api/backend/organization/${orgId}/containers`
})

const { data, status, refresh: refreshData } = await useLazyFetch<Container[]>(
  fetchUrl,
  {
    key: 'project-resources',
    query: {
      project_id: projectId,
      environment_id: environmentId,
    },
    immediate: computed(() => !!(store.organization?.id && projectId.value && environmentId.value)),
  },
)

const containers = computed<ContainerWithProject[]>(() => {
  return (data.value ?? []).map(c => ({
    ...c,
    _projectName: projectName.value,
    _projectId: projectId.value ?? undefined,
  }))
})

function refresh() { refreshData() }

watch(() => store.refreshKey, () => { refreshData() })
</script>

<template>
  <div class="flex w-full max-w-[1500px] flex-col gap-5 mx-auto">
    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div class="min-w-0">
        <p class="mb-2 truncate text-sm text-muted">{{ projectName }} <span class="mx-1 text-default/30">/</span> {{ environmentName }}</p>
        <h1 class="text-2xl font-semibold">Containers</h1>
        <p class="mt-1 text-sm text-muted">Runtime services deployed to this environment.</p>
      </div>
      <UButton class="shrink-0" :icon="ICONS.plus" :to="`/${route.params.organization_slug}/containers/${projectId}/${environmentId}/new`">New Container</UButton>
    </div>

    <div class="grid overflow-hidden rounded-md border border-dashed border-default bg-transparent sm:grid-cols-3">
      <div class="px-4 py-3"><p class="text-xs text-muted">Environment</p><p class="mt-1 text-sm font-medium">{{ environmentName }}</p></div>
      <div class="px-4 py-3"><p class="text-xs text-muted">Containers</p><p class="mt-1 font-mono text-sm">{{ containers.length }}</p></div>
      <div class="px-4 py-3"><p class="text-xs text-muted">Desired replicas</p><p class="mt-1 font-mono text-sm">{{ containers.reduce((total, container) => total + (container.current_version?.replica_count ?? 0), 0) }}</p></div>
    </div>

    <DeploymentsContainersListing
      v-if="projectId && environmentId && store.organization"
      :containers="containers"
      :organization-id="store.organization.id"
      :project-id="projectId"
      :environment-id="environmentId"
      :status="status"
      @refresh="refresh"
    />

  </div>
</template>
