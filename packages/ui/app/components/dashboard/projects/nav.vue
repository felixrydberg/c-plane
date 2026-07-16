<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const router = useRouter()
const toast = useToast()

const routeProjectId = computed(() => route.params.project_id as string | undefined)
const routeBranchId = computed(() => route.params.branch_id as string | undefined)

const PROJECT_PAGES = ['containers', 'databases/stateful', 'secrets']
const BRANCH_PAGES = ['containers']

const currentSection = computed(() => {
  const pathAfterSlug = route.path.slice((store.organization?.slug?.length ?? 0) + 1)
  const pid = routeProjectId.value
  const segment = pid ? pathAfterSlug.slice(0, pathAfterSlug.indexOf(pid) - 1) : pathAfterSlug
  return segment.replace(/^\/+|\/+$/g, '')
})
const projectRoutesEnabled = computed(() => PROJECT_PAGES.includes(currentSection.value))
const branchRoutesEnabled = computed(() => BRANCH_PAGES.includes(currentSection.value))

// Projects are loaded by the auth plugin on every request.
// Refresh only needed after create/delete.
async function refreshProjects() {
  if (!store.organization?.id) return
  const { data } = await $fetch<{ data: { id: string; organization_id: string; name: string; default_branch_id: string | null }[] }>(
    `/api/backend/organization/${store.organization.id}/projects`
  )
  if (data) {
    store.projects = data.map(p => ({
      id: p.id, organization_id: p.organization_id, name: p.name, default_branch_id: p.default_branch_id,
    }))
  }
}

watch([() => store.projects, routeProjectId], ([projList, pid]) => {
  if (!pid) return
  if (store.project && store.project.id === pid) return
  const matched = projList.find(p => p.id === pid)
  if (matched) store.project = matched
}, { immediate: true })

type BranchItem = { id: string; name: string; timeline: string; is_default: boolean }
type BranchResponse = BranchItem[] | { data: BranchItem[] }

const { data: branchesData, refresh: refreshBranches, pending: branchesPending } = await useFetch<BranchResponse>(
  () => !!store.organization?.id && !!store.project?.id ? `/api/backend/organization/${store.organization!.id}/projects/${store.project!.id}/branches` : '',
  { immediate: computed(() => !!(store.organization?.id && store.project?.id)) },
)

const branches = computed(() => {
  const value = branchesData.value
  return Array.isArray(value) ? value : value?.data ?? []
})

watch(branches, (val) => {
  store.branches = val
  if (val.length) {
    const target = val.find(b => b.id === routeBranchId.value) ?? val.find(b => b.is_default) ?? val[0]
    if (target) store.branch = { id: target.id, name: target.name, timeline: target.timeline, is_default: target.is_default }
  }
}, { immediate: true })

const createProjectModal = ref(false)
const deleteProjectModal = ref(false)
const createBranchModal = ref(false)
const deleteBranchModal = ref(false)

const projectLabel = computed(() => store.project?.name || 'All Projects')

const projectItems = computed<DropdownMenuItem[][]>(() => {
  if (!store.projects.length) return [[
    { label: 'No projects available', disabled: true },
  ], [
    { label: 'Create Project', icon: ICONS.folderPlus, onSelect() { createProjectModal.value = true } },
  ]]

  const list: DropdownMenuItem[] = [
    { label: 'All Projects', icon: ICONS.globeAlt, onSelect() { selectProject(null) } },
  ]
  for (const p of store.projects) {
    list.push({ label: p.name, icon: ICONS.folder, onSelect() { selectProject(p.id) } })
  }

  const actions: DropdownMenuItem[] = [
    { label: 'Create Project', icon: ICONS.folderPlus, onSelect() { createProjectModal.value = true } },
  ]
  if (store.project) {
    actions.push({ label: 'Delete Project', icon: 'i-heroicons:trash', color: 'error' as const, onSelect() { deleteProjectModal.value = true } })
  }
  return [list, actions]
})

const branchLabel = computed(() => {
  if (!store.project) return 'Select branch'
  if (branchesPending.value) return 'Loading...'
  if (!store.branch) return store.branches.length ? 'Select branch' : 'No branches'
  return store.branch.name
})

const branchItems = computed<DropdownMenuItem[][]>(() => {
  if (!store.project) return [[{ label: 'Select a project first', disabled: true }]]
  if (branchesPending.value) return [[{ label: 'Loading branches...', disabled: true }]]
  if (!store.branches.length) return [[
    { label: 'No branches', disabled: true },
  ], [
    { label: 'Create Branch', icon: ICONS.folderPlus, onSelect() { createBranchModal.value = true } },
  ]]

  const list: DropdownMenuItem[] = store.branches.map(b => ({
    label: b.name + (b.is_default ? ' (default)' : ''),
    icon: ICONS.folder,
    onSelect() { selectBranch(b) },
  }))
  const actions: DropdownMenuItem[] = [
    { label: 'Create Branch', icon: ICONS.folderPlus, onSelect() { createBranchModal.value = true } },
  ]
  if (store.branch) {
    actions.push({
      label: 'Delete Branch',
      icon: 'i-heroicons:trash',
      color: 'error' as const,
      disabled: store.branch.is_default,
      description: store.branch.is_default ? 'Cannot delete the default branch' : undefined,
      onSelect() {
        if (!store.branch?.is_default) deleteBranchModal.value = true;
      },
    })
  }
  return [list, actions]
})

async function selectProject(projectId: string | null) {
  const slug = store.organization?.slug
  if (!slug) return

  const current = routeProjectId.value
  if (projectId === current) return

  if (!projectRoutesEnabled.value) {
    store.branches = []
    if (projectId) {
      store.project = store.projects.find(p => p.id === projectId) ?? null
    } else {
      store.project = null
      store.branch = null
    }
    return
  }

  const pathAfterSlug = route.path.slice(slug.length + 1)
  const baseSection = (current
    ? pathAfterSlug.slice(0, pathAfterSlug.indexOf(current) - 1)
    : pathAfterSlug)
    .replace(/^\/+|\/+$/g, '')
  if (!baseSection) return

  if (projectId) {
    store.project = store.projects.find(p => p.id === projectId) ?? null
    router.push(`/${slug}/${baseSection}/${projectId}`)
  } else {
    store.project = null
    store.branch = null
    store.branches = []
    router.push(`/${slug}`)
  }
}

function selectBranch(b: BranchItem) {
  store.branch = { id: b.id, name: b.name, timeline: b.timeline, is_default: b.is_default }

  const slug = store.organization?.slug
  const pid = routeProjectId.value
  if (!pid || !slug || !projectRoutesEnabled.value) return
  if (b.id === routeBranchId.value) return

  const pathAfterSlug = route.path.slice(slug.length + 1)
  const baseSection = pathAfterSlug.slice(0, pathAfterSlug.indexOf(pid) - 1).replace(/^\/+|\/+$/g, '')
  const url = branchRoutesEnabled.value
    ? `/${slug}/${baseSection}/${pid}/${b.id}`
    : `/${slug}/${baseSection}/${pid}`
  router.push(url)
}

async function onProjectCreated() { await refreshProjects() }
async function onProjectDeleted() { await refreshProjects(); selectProject(null) }
async function onBranchCreated() { await refreshBranches() }

async function onConfirmDeleteBranch() {
  if (!store.organization?.id || !store.project?.id || !store.branch) return;
  try {
    await $fetch(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/branches/${store.branch.id}`,
      { method: 'DELETE' }
    );
    toast.add({ title: 'Branch removed', color: 'success' });
    deleteBranchModal.value = false;
    store.branch = null;
    await refreshBranches();

    const list = branches.value
    store.branches = list
    const target = list.find(b => b.id === routeBranchId.value) ?? list.find(b => b.is_default) ?? list[0] ?? null
    if (target) {
      store.branch = { id: target.id, name: target.name, timeline: target.timeline, is_default: target.is_default }
      const slug = store.organization?.slug
      const pid = routeProjectId.value
      if (pid && slug && projectRoutesEnabled.value) {
        const pathAfterSlug = route.path.slice(slug.length + 1)
        const baseSection = pathAfterSlug.slice(0, pathAfterSlug.indexOf(pid) - 1).replace(/^\/+|\/+$/g, '')
        const url = branchRoutesEnabled.value
          ? `/${slug}/${baseSection}/${pid}/${target.id}`
          : `/${slug}/${baseSection}/${pid}`
        router.push(url)
      }
    }
  } catch {
    toast.add({ title: 'Failed to remove branch', color: 'error' });
  }
}

const graphModalOpen = ref(false)
</script>

<template>
  <div class="flex items-center gap-2">
    <UDropdownMenu size="sm" :items="projectItems" :content="{ align: 'start', collisionPadding: 12 }" :ui="{ content: 'w-64' }">
      <UButton :label="projectLabel" :trailing-icon="ICONS.chevronUpDown" size="sm" color="neutral" variant="soft" class="data-[state=open]:bg-elevated" :ui="{ trailingIcon: 'text-dimmed' }" />
    </UDropdownMenu>

    <USeparator orientation="vertical" class="h-6" />

    <UDropdownMenu size="sm" :items="branchItems" :content="{ align: 'start', collisionPadding: 12 }" :ui="{ content: 'w-64' }">
      <UButton :label="branchLabel" :trailing-icon="ICONS.chevronUpDown" size="sm" color="neutral" variant="soft" :disabled="!store.project" class="data-[state=open]:bg-elevated" :ui="{ trailingIcon: 'text-dimmed' }" />
    </UDropdownMenu>

    <UButton v-if="store.project" :icon="ICONS.graph" variant="ghost" color="neutral" size="sm" class="rotate-180" aria-label="Branch graph" @click="graphModalOpen = true" />

    <DashboardProjectsCreateModal v-model:open="createProjectModal" @created="onProjectCreated" />
    <DashboardProjectsDeleteModal v-model:open="deleteProjectModal" @deleted="onProjectDeleted" />
    <DashboardProjectsCreateBranchModal v-model:open="createBranchModal" @created="onBranchCreated" />
    <DashboardProjectsBranchGraphModal v-model:open="graphModalOpen" />

    <UModal v-model:open="deleteBranchModal" title="Delete Branch" :ui="{ content: 'max-w-sm' }">
      <template #body>
        <p class="text-sm">
          Are you sure you want to delete the branch <strong class="capitalize">{{ store.branch?.name }}</strong>? Timeline revisions will be preserved and can be repointed to.
        </p>
        <div class="flex justify-end gap-3 pt-4">
          <UButton variant="ghost" color="neutral" @click="deleteBranchModal = false">Cancel</UButton>
          <UButton color="error" :icon="ICONS.trash" @click="onConfirmDeleteBranch">Delete</UButton>
        </div>
      </template>
    </UModal>
  </div>
</template>
