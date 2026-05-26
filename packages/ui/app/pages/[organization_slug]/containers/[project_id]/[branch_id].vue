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
const createModalOpen = ref(false)

const projectName = computed(() => {
  return store.projects.find(p => p.id === projectId.value)?.name ?? projectId.value ?? ''
})

const branchName = computed(() => {
  if (!branchId.value) return 'default'
  return store.branches.find(b => b.id === branchId.value)?.name ?? branchId.value
})

useHead({
  title: `Containers - ${projectName.value}/${branchName.value} - C-Plane`,
})

const fetchUrl = computed(() => {
  const orgId = store.organization?.id
  if (!orgId || !projectId.value || !branchId.value) return ''
  return `/api/backend/organization/${orgId}/containers`
})

const { data, status, refresh: refreshData } = await useLazyFetch<Container[]>(
  fetchUrl,
  {
    query: computed(() => ({
      project_id: projectId.value,
      branch_id: branchId.value,
    })),
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
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-semibold">Containers</h1>
        <p class="text-muted text-sm mt-1">
          Manage container deployments for {{ projectName }} / {{ branchName }}.
        </p>
      </div>
      <UButton :icon="ICONS.plus" @click="createModalOpen = true">New Container</UButton>
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

    <DeploymentsContainersCreateModal
      v-model:open="createModalOpen"
      :organization-id="store.organization?.id ?? ''"
      @created="refresh"
    />
  </div>
</template>
