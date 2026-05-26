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

interface BranchListing {
  projectName: string
  projectId: string
  branchName: string
  branchId: string
  containers: ContainerWithProject[]
}

const store = useStore()
const createModalOpen = ref(false)

useHead({ title: `Containers - All Projects - C-Plane` })

const globalListings = ref<BranchListing[]>([])
const globalLoading = ref(true)
const globalError = ref('')

async function fetchGlobalView() {
  if (!store.organization?.id) return
  globalLoading.value = true
  globalError.value = ''
  globalListings.value = []

  try {
    const branches = await $fetch<{
      id: string; name: string; timeline: string; is_default: boolean;
      project_id: string; project_name: string;
    }[]>(
      `/api/backend/organization/${store.organization.id}/branches`
    )

    for (const branch of branches) {
      const branchContainers = await $fetch<Container[]>(
        `/api/backend/organization/${store.organization.id}/containers`,
        { query: { branch_id: branch.id } }
      )

      if (branchContainers.length > 0) {
        globalListings.value.push({
          projectName: branch.project_name,
          projectId: branch.project_id,
          branchName: branch.name,
          branchId: branch.id,
          containers: branchContainers.map(c => ({
            ...c,
            _projectName: branch.project_name,
            _projectId: branch.project_id,
          })),
        })
      }
    }
  } catch {
    globalError.value = 'Failed to load containers'
  } finally {
    globalLoading.value = false
  }
}

onMounted(() => { fetchGlobalView() })

function refresh() { fetchGlobalView() }
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-semibold">Containers</h1>
        <p class="text-muted text-sm mt-1">Manage container deployments across all projects.</p>
      </div>
      <UButton :icon="ICONS.plus" @click="createModalOpen = true">New Container</UButton>
    </div>

    <div v-if="globalLoading" class="text-center py-8"><UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" /></div>
    <p v-else-if="globalError" class="text-sm text-red-500">{{ globalError }}</p>

    <div
      v-else-if="globalListings.length === 0"
      class="flex flex-col items-center justify-center py-16 gap-3 text-center border border-dashed border-default rounded-lg"
    >
      <UIcon :name="ICONS.containers" class="size-10 text-muted" />
      <p class="text-muted">Select a project or create a container to get started.</p>
    </div>

    <template v-else>
      <div
        v-for="listing in globalListings"
        :key="`${listing.projectId}-${listing.branchId}`"
      >
        <h2 class="text-lg font-medium mb-3">
          {{ listing.projectName }}
          <span class="text-muted font-normal">/ {{ listing.branchName }}</span>
        </h2>
        <DeploymentsContainersListing
          :containers="listing.containers"
          :organization-id="store.organization!.id"
          :project-id="listing.projectId"
          :branch-id="listing.branchId"
          status="success"
          @refresh="refresh"
        />
      </div>
    </template>

    <DeploymentsContainersCreateModal
      v-model:open="createModalOpen"
      :organization-id="store.organization?.id ?? ''"
      @created="refresh"
    />
  </div>
</template>
