<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import { loadProjectEnvironments } from '~/utils/auth'
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const router = useRouter()
const toast = useToast()

const routeProjectId = computed(() => route.params.project_id as string | undefined)
const routeEnvironmentId = computed(() => route.params.environment_id as string | undefined)

const PROJECT_PAGES = ['containers', 'databases/postgres', 'secrets']
const ENVIRONMENT_PAGES = ['containers']

const currentSection = computed(() => {
  const pathAfterSlug = route.path.slice((store.organization?.slug?.length ?? 0) + 1)
  const pid = routeProjectId.value
  const segment = pid ? pathAfterSlug.slice(0, pathAfterSlug.indexOf(pid) - 1) : pathAfterSlug
  return segment.replace(/^\/+|\/+$/g, '')
})
const projectRoutesEnabled = computed(() => PROJECT_PAGES.includes(currentSection.value))
const environmentRoutesEnabled = computed(() => ENVIRONMENT_PAGES.includes(currentSection.value))

// Projects are loaded by the auth plugin on every request.
// Refresh only needed after create/delete.
async function refreshProjects() {
  if (!store.organization?.id) return
  const { data } = await $fetch<{ data: { id: string; organization_id: string; name: string; default_environment_id: string | null }[] }>(
    `/api/backend/organization/${store.organization.id}/projects`
  )
  if (data) {
    store.projects = data.map(p => ({
      id: p.id, organization_id: p.organization_id, name: p.name, default_environment_id: p.default_environment_id,
    }))
  }
}

watch([() => store.projects, routeProjectId], ([projList, pid]) => {
  if (!pid) return
  if (store.project && store.project.id === pid) return
  const matched = projList.find(p => p.id === pid)
  if (matched) store.project = matched
}, { immediate: true })

type EnvironmentItem = { id: string; name: string; timeline: string; is_default: boolean; has_recent_undeployed_revision: boolean }

const createProjectModal = ref(false)
const deleteProjectModal = ref(false)
const createEnvironmentModal = ref(false)
const deleteEnvironmentModal = ref(false)

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

const environmentLabel = computed(() => {
  if (!store.project) return 'Select environment'
  if (!store.environment) return store.environments.length ? 'Select environment' : 'No environments'
  return store.environment.name
})
const environmentItems = computed<DropdownMenuItem[][]>(() => {
  if (!store.project) return [[{ label: 'Select a project first', disabled: true }]]
  if (!store.environments.length) return [[
    { label: 'No environments', disabled: true },
  ], [
    { label: 'Create Environment', icon: ICONS.folderPlus, onSelect() { createEnvironmentModal.value = true } },
  ]]

  const list: DropdownMenuItem[] = store.environments.map(b => ({
    label: b.name + (b.is_default ? ' (default)' : ''),
    icon: ICONS.folder,
    onSelect() { selectEnvironment(b) },
  }))
  const actions: DropdownMenuItem[] = [
    { label: 'Create Environment', icon: ICONS.folderPlus, onSelect() { createEnvironmentModal.value = true } },
  ]
  if (store.environment) {
    actions.push({
      label: 'Delete Environment',
      icon: 'i-heroicons:trash',
      color: 'error' as const,
      disabled: store.environment.is_default,
      description: store.environment.is_default ? 'Cannot delete the default environment' : undefined,
      onSelect() {
        if (!store.environment?.is_default) deleteEnvironmentModal.value = true;
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
    store.environments = []
    store.environments_project_id = null
    if (projectId) {
      store.project = store.projects.find(p => p.id === projectId) ?? null
      await loadProjectEnvironments(projectId)
    } else {
      store.project = null
      store.environment = null
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
    store.environment = null
    store.environments = []
    store.environments_project_id = null
    router.push(`/${slug}/${baseSection}/${projectId}`)
  } else {
    store.project = null
    store.environment = null
    store.environments = []
    store.environments_project_id = null
    router.push(`/${slug}`)
  }
}

function selectEnvironment(b: EnvironmentItem) {
  store.environment = { id: b.id, name: b.name, timeline: b.timeline, is_default: b.is_default, has_recent_undeployed_revision: b.has_recent_undeployed_revision }

  const slug = store.organization?.slug
  const pid = routeProjectId.value
  if (!pid || !slug || !projectRoutesEnabled.value) return
  if (b.id === routeEnvironmentId.value) return

  const pathAfterSlug = route.path.slice(slug.length + 1)
  const baseSection = pathAfterSlug.slice(0, pathAfterSlug.indexOf(pid) - 1).replace(/^\/+|\/+$/g, '')
  const url = environmentRoutesEnabled.value
    ? `/${slug}/${baseSection}/${pid}/${b.id}`
    : `/${slug}/${baseSection}/${pid}`
  router.push(url)
}

async function onProjectCreated() { await refreshProjects() }
async function onProjectDeleted() { await refreshProjects(); selectProject(null) }
async function refreshEnvironments() {
  if (!store.project) return
  await loadProjectEnvironments(store.project.id, routeEnvironmentId.value)
}

async function onEnvironmentCreated() { await refreshEnvironments() }

async function onConfirmDeleteEnvironment() {
  if (!store.organization?.id || !store.project?.id || !store.environment) return;
  try {
    await $fetch(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/environments/${store.environment.id}`,
      { method: 'DELETE' }
    );
    toast.add({ title: 'Environment removed', color: 'success' });
    deleteEnvironmentModal.value = false;
    store.environment = null;
    await refreshEnvironments();

    const list = store.environments
    const target = list.find(b => b.id === routeEnvironmentId.value) ?? list.find(b => b.is_default) ?? list[0] ?? null
    if (target) {
      store.environment = { id: target.id, name: target.name, timeline: target.timeline, is_default: target.is_default, has_recent_undeployed_revision: target.has_recent_undeployed_revision }
      const slug = store.organization?.slug
      const pid = routeProjectId.value
      if (pid && slug && projectRoutesEnabled.value) {
        const pathAfterSlug = route.path.slice(slug.length + 1)
        const baseSection = pathAfterSlug.slice(0, pathAfterSlug.indexOf(pid) - 1).replace(/^\/+|\/+$/g, '')
        const url = environmentRoutesEnabled.value
          ? `/${slug}/${baseSection}/${pid}/${target.id}`
          : `/${slug}/${baseSection}/${pid}`
        router.push(url)
      }
    }
  } catch {
    toast.add({ title: 'Failed to remove environment', color: 'error' });
  }
}

const graphModalOpen = ref(false)
</script>

<template>
  <div class="flex min-w-0 items-center gap-2">
    <UDropdownMenu size="sm" :items="projectItems" :content="{ align: 'start', collisionPadding: 12 }" :ui="{ content: 'w-64' }" class="shrink-0">
      <UButton :label="projectLabel" :trailing-icon="ICONS.chevronUpDown" size="sm" color="neutral" variant="soft" class="data-[state=open]:bg-elevated" :ui="{ trailingIcon: 'text-dimmed' }" />
    </UDropdownMenu>

    <USeparator orientation="vertical" class="h-6 shrink-0" />

    <UDropdownMenu size="sm" :items="environmentItems" :content="{ align: 'start', collisionPadding: 12 }" :ui="{ content: 'w-64' }" class="shrink-0">
      <UButton :label="environmentLabel" :trailing-icon="ICONS.chevronUpDown" size="sm" color="neutral" variant="soft" :disabled="!routeProjectId && !store.project" class="data-[state=open]:bg-elevated" :ui="{ trailingIcon: 'text-dimmed' }" />
    </UDropdownMenu>
    <UButton v-if="store.project" :icon="ICONS.graph" variant="ghost" color="neutral" size="sm" class="shrink-0 rotate-180" aria-label="Environment graph" @click="graphModalOpen = true" />

    <DashboardProjectsCreateModal v-model:open="createProjectModal" @created="onProjectCreated" />
    <DashboardProjectsDeleteModal v-model:open="deleteProjectModal" @deleted="onProjectDeleted" />
    <DashboardProjectsCreateEnvironmentModal v-model:open="createEnvironmentModal" @created="onEnvironmentCreated" />
    <DashboardProjectsEnvironmentGraphModal v-model:open="graphModalOpen" />

    <UModal v-model:open="deleteEnvironmentModal" title="Delete Environment" :ui="{ content: 'max-w-sm' }">
      <template #body>
        <p class="text-sm">
          Are you sure you want to delete the environment <strong class="capitalize">{{ store.environment?.name }}</strong>? Timeline revisions will be preserved and can be repointed to.
        </p>
        <div class="flex justify-end gap-3 pt-4">
          <UButton variant="ghost" color="neutral" @click="deleteEnvironmentModal = false">Cancel</UButton>
          <UButton color="error" :icon="ICONS.trash" @click="onConfirmDeleteEnvironment">Delete</UButton>
        </div>
      </template>
    </UModal>
  </div>
</template>
