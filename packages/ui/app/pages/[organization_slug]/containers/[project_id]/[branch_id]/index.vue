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
const branchId = computed(() => route.params.branch_id?.toString() || null)

type BranchItem = { id: string; name: string; timeline: string; is_default: boolean }

const orgId = computed(() => store.organization?.id)

const projectName = computed(() =>
  store.projects.find(p => p.id === projectId.value)?.name ?? projectId.value ?? ''
)

const { data: branchList } = await useFetch<BranchItem[]>(
  () => orgId.value && projectId.value ? `/api/backend/organization/${orgId.value}/projects/${projectId.value}/branches` : '',
  { immediate: computed(() => !!(orgId.value && projectId.value)) },
)

const branchName = computed(() => {
  if (!branchId.value) return 'default'
  const list = branchList.value ?? store.branches
  return list.find(b => b.id === branchId.value)?.name ?? branchId.value
})

const fetchUrl = computed(() => {
  const orgId = store.organization?.id
  if (!orgId || !projectId.value || !branchId.value) return ''
  return `/api/backend/organization/${orgId}/containers`
})

const { data, status, refresh: refreshData } = await useLazyFetch<Container[]>(
  fetchUrl,
  {
    key: 'project-resources',
    query: {
      project_id: projectId,
      branch_id: branchId,
    },
    immediate: computed(() => !!(store.organization?.id && projectId.value && branchId.value)),
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
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div class="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
      <div class="min-w-0">
        <p class="mb-2 truncate text-sm text-muted">{{ projectName }} <span class="mx-1 text-default/30">/</span> {{ branchName }}</p>
        <h1 class="text-2xl font-semibold">Containers</h1>
        <p class="mt-1 text-sm text-muted">Services deployed to this branch.</p>
      </div>
      <UButton class="shrink-0" :icon="ICONS.plus" :to="`/${route.params.organization_slug}/containers/${projectId}/${branchId}/new`">New Container</UButton>
    </div>

    <DeploymentsContainersListing
      v-if="projectId && branchId && store.organization"
      :containers="containers"
      :organization-id="store.organization.id"
      :project-id="projectId"
      :branch-id="branchId"
      :status="status"
      @refresh="refresh"
    />

  </div>
</template>
