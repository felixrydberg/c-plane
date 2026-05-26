<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const router = useRouter()

const routeProjectId = computed(() => route.params.project_id as string | undefined)
const routeBranchId = computed(() => route.params.branch_id as string | undefined)

const PROJECT_PAGES = ['containers', 'databases/stateful', 'secrets']

const currentSection = computed(() => {
  const pathAfterSlug = route.path.slice((store.organization?.slug?.length ?? 0) + 1)
  const pid = routeProjectId.value
  const segment = pid ? pathAfterSlug.slice(0, pathAfterSlug.indexOf(pid) - 1) : pathAfterSlug
  return segment.replace(/^\/+|\/+$/g, '')
})
const projectRoutesEnabled = computed(() => PROJECT_PAGES.includes(currentSection.value))

const projects = ref<{ id: string; organization_id: string; name: string; default_branch_id: string | null }[]>([])
const projectsPending = ref(false)

type BranchItem = { id: string; name: string; timeline: string; is_default: boolean }
const branches = ref<BranchItem[]>([])
const branchesPending = ref(false)

async function fetchProjects() {
  if (!store.organization?.id) return
  projectsPending.value = true
  try {
    const res = await $fetch<{ data: { id: string; organization_id: string; name: string; default_branch_id: string | null }[] }>(
      `/api/backend/organization/${store.organization.id}/projects`
    )
    projects.value = (res.data ?? []).map(p => ({
      id: p.id, organization_id: p.organization_id, name: p.name, default_branch_id: p.default_branch_id,
    }))
    store.projects = projects.value
  } catch { projects.value = [] } finally { projectsPending.value = false }
}

async function fetchBranches() {
  if (!store.organization?.id || !store.project?.id) return
  branchesPending.value = true
  try {
    const data = await $fetch<BranchItem[]>(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/branches`
    )
    branches.value = data
    store.branches = data
    const target =
      data.find(b => b.id === routeBranchId.value) ??
      data.find(b => b.is_default) ??
      data[0] ??
      null
    if (target) {
      store.branch = { id: target.id, name: target.name, timeline: target.timeline, is_default: target.is_default }
    }
  } catch { branches.value = [] } finally { branchesPending.value = false }
}

async function selectProject(projectId: string | null) {
  const slug = store.organization?.slug
  if (!slug) return

  const current = routeProjectId.value
  if (projectId === current) return

  if (!projectRoutesEnabled.value) {
    store.branches = []
    branches.value = []
    if (projectId) {
      store.project = projects.value.find(p => p.id === projectId)
        ?? store.projects.find(p => p.id === projectId)
        ?? null
      await fetchBranches()
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
    store.project = projects.value.find(p => p.id === projectId)
      ?? store.projects.find(p => p.id === projectId)
      ?? null
    router.push(`/${slug}/${baseSection}/${projectId}`)
    await fetchBranches()
  } else {
    store.project = null
    store.branch = null
    store.branches = []
    branches.value = []
    router.push(`/${slug}/${baseSection}`)
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
  router.push(`/${slug}/${baseSection}/${pid}/${b.id}`)
}

await fetchProjects()
if (routeProjectId.value && store.project) {
  await fetchBranches()
}

const createProjectModal = ref(false)
const deleteProjectModal = ref(false)
const createBranchModal = ref(false)
const historyBranchModal = ref(false)

const projectLabel = computed(() => store.project?.name || 'All Projects')

const projectItems = computed<DropdownMenuItem[][]>(() => {
  if (projectsPending.value) return [[{ label: 'Loading projects...', disabled: true }]]
  if (!projects.value.length) return [[
    { label: 'No projects available', disabled: true },
  ], [
    { label: 'Create Project', icon: ICONS.folderPlus, onSelect() { createProjectModal.value = true } },
  ]]

  const list: DropdownMenuItem[] = [
    { label: 'All Projects', icon: ICONS.globeAlt, onSelect() { selectProject(null) } },
  ]
  for (const p of projects.value) {
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
  if (!store.branch) return branches.value.length ? 'Select branch' : 'No branches'
  return store.branch.name
})

const branchItems = computed<DropdownMenuItem[][]>(() => {
  if (!store.project) return [[{ label: 'Select a project first', disabled: true }]]
  if (branchesPending.value) return [[{ label: 'Loading branches...', disabled: true }]]
  if (!branches.value.length) return [[
    { label: 'No branches', disabled: true },
  ], [
    { label: 'Create Branch', icon: ICONS.folderPlus, onSelect() { createBranchModal.value = true } },
  ]]

  const list: DropdownMenuItem[] = branches.value.map(b => ({
    label: b.name + (b.is_default ? ' (default)' : ''),
    icon: ICONS.folder,
    onSelect() { selectBranch(b) },
  }))
  const actions: DropdownMenuItem[] = [
    { label: 'Create Branch', icon: ICONS.folderPlus, onSelect() { createBranchModal.value = true } },
  ]
  if (store.branch) {
    actions.push({ label: 'Branch History', icon: 'i-heroicons:clock', onSelect() { historyBranchModal.value = true } })
  }
  return [list, actions]
})

async function onProjectCreated() { await fetchProjects() }
async function onProjectDeleted() { await fetchProjects(); selectProject(null) }
async function onBranchCreated() { await fetchBranches() }
async function onBranchUpdated() { await fetchBranches() }

const graphModalOpen = ref(false)
</script>

<template>
  <div class="flex items-center gap-2">
    <UDropdownMenu :items="projectItems" :content="{ align: 'start', collisionPadding: 12 }" :ui="{ content: 'w-64' }">
      <UButton :label="projectLabel" :trailing-icon="ICONS.chevronUpDown" color="neutral" variant="soft" class="data-[state=open]:bg-elevated" :ui="{ trailingIcon: 'text-dimmed' }" />
    </UDropdownMenu>

    <USeparator orientation="vertical" class="h-6" />

    <UDropdownMenu :items="branchItems" :content="{ align: 'start', collisionPadding: 12 }" :ui="{ content: 'w-64' }">
      <UButton :label="branchLabel" :trailing-icon="ICONS.chevronUpDown" color="neutral" variant="soft" :disabled="!store.project" class="data-[state=open]:bg-elevated" :ui="{ trailingIcon: 'text-dimmed' }" />
    </UDropdownMenu>

    <UButton v-if="store.project" icon="i-heroicons:presentation-chart-line" variant="ghost" color="neutral" size="sm" aria-label="Branch graph" @click="graphModalOpen = true" />

    <DashboardProjectsCreateModal v-model:open="createProjectModal" @created="onProjectCreated" />
    <DashboardProjectsDeleteModal v-model:open="deleteProjectModal" @deleted="onProjectDeleted" />
    <DashboardProjectsCreateBranchModal v-model:open="createBranchModal" @created="onBranchCreated" />
    <DashboardProjectsBranchHistoryModal v-model:open="historyBranchModal" @updated="onBranchUpdated" />
    <DashboardProjectsBranchGraphModal v-model:open="graphModalOpen" />
  </div>
</template>
