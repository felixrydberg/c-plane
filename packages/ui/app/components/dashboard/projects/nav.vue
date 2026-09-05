<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import type { Environment } from '@cplane/sdk'
import { loadProjectEnvironments } from '~/utils/auth'
import { syncEnvironment } from '~/utils/environments'
import { ICONS } from '~/utils/icons'

const props = withDefaults(defineProps<{ collapsed?: boolean }>(), {
  collapsed: false,
})

const store = useStore()
const route = useRoute()
const router = useRouter()
const toast = useToast()
const isOwner = computed(() => store.organization?.member?.role === 'owner')

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
const isViewingDeployed = computed(() =>
  route.query.revision === store.environment?.deployed_timeline
)

// Projects are loaded by the auth plugin on every request.
// Refresh only needed after create/delete.
async function refreshProjects() {
  if (!store.organization?.id) return
  const { data } = await cplaneFetch(`/api/organization/${store.organization.id as ':organization_id'}/projects` as const)
  if (data) {
    store.projects = data
  }
}

const createProjectModal = ref(false)
const deleteProjectModal = ref(false)
const createEnvironmentModal = ref(false)
const deleteEnvironmentModal = ref(false)
const renameEnvironmentModal = ref(false)
const environmentName = ref('')
const renamingEnvironment = ref(false)
const graphModalOpen = ref(false)

const projectLabel = computed(() => store.project?.name || 'All Projects')

const environmentLabel = computed(() => {
  if (!store.project) return 'Select environment'
  if (!store.environment) return store.environments.length ? 'Select environment' : 'No environments'
  return store.environment.name
})

const projectMenuItems = computed<DropdownMenuItem[][]>(() => {
  const projects: DropdownMenuItem[] = [
    { label: 'All Projects', onSelect() { void selectProject(null) } },
    ...store.projects.map(project => ({
      label: project.name,
      onSelect() { void selectProject(project.id) },
    })),
  ]
  const actions: DropdownMenuItem[] = [
  ...(isOwner.value ? [{ label: 'Create Project', icon: ICONS.folderPlus, onSelect() { createProjectModal.value = true } }] : []),
  ...(store.project && isOwner.value ? [{ label: 'Delete Project', icon: ICONS.trash, color: 'error' as const, onSelect() { deleteProjectModal.value = true } }] : []),
  ]
  return actions.length ? [projects, actions] : [projects]
})

const environmentMenuItems = computed<DropdownMenuItem[][]>(() => {
  const environments: DropdownMenuItem[] = store.environments.map(environment => ({
      label: environment.name,
      badges: [
        ...(environment.is_default ? ['Default'] : []),
        ...(environment.is_preview ? ['Preview'] : []),
      ],
      onSelect() { selectEnvironment(environment) },
    }))
  const actions: DropdownMenuItem[] = [
      ...(isOwner.value ? [{ label: 'Create Environment', icon: ICONS.folderPlus, onSelect() { createEnvironmentModal.value = true } }] : []),
      ...(store.environment && isOwner.value ? [{
        label: 'Rename Environment',
        icon: ICONS.pencil,
        onSelect() {
          environmentName.value = store.environment?.name ?? ''
          renameEnvironmentModal.value = true
        },
      }] : []),
      ...(store.environment ? [{
        label: 'Delete Environment',
        icon: ICONS.trash,
        color: 'error' as const,
        disabled: !isOwner.value || store.environment.is_default,
        description: !isOwner.value ? 'Owner role required' : undefined,
        onSelect() {
          if (!store.environment?.is_default) deleteEnvironmentModal.value = true
        },
      }] : []),
    ]
  return actions.length ? [environments, actions] : [environments]
})

async function selectProject(projectId: string | null) {
  const slug = store.organization?.slug
  if (!slug) return

  const current = routeProjectId.value
  if (projectId === current) return

  if (!projectRoutesEnabled.value) {
    if (projectId) {
      await loadProjectEnvironments(projectId)
    } else {
      store.$patch({ project: null, environment: null, environments: [], environments_project_id: null })
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
    router.push(`/${slug}/${baseSection}/${projectId}`)
  } else {
    store.$patch({ project: null, environment: null, environments: [], environments_project_id: null })
    router.push(`/${slug}`)
  }
}

function selectEnvironment(b: Environment) {
  const slug = store.organization?.slug
  const pid = routeProjectId.value
  if (!pid || !slug || !projectRoutesEnabled.value || !environmentRoutesEnabled.value || b.id === routeEnvironmentId.value) {
    store.environment = b
    return
  }

  const pathAfterSlug = route.path.slice(slug.length + 1)
  const baseSection = pathAfterSlug.slice(0, pathAfterSlug.indexOf(pid) - 1).replace(/^\/+|\/+$/g, '')
  const url = environmentRoutesEnabled.value
    ? `/${slug}/${baseSection}/${pid}/${b.id}`
    : `/${slug}/${baseSection}/${pid}`
  router.push(`${url}?revision=${isViewingDeployed.value ? b.deployed_timeline : b.draft_timeline}`)
}

async function onProjectCreated() { await refreshProjects() }
async function onProjectDeleted() {
  await refreshProjects()
  if (!store.projects.length && store.organization?.slug) {
    await router.push(`/${store.organization.slug}/onboarding`)
    return
  }
  await selectProject(null)
}
async function refreshEnvironments() {
  if (!store.project) return
  await loadProjectEnvironments(store.project.id, routeEnvironmentId.value)
}

async function onEnvironmentCreated() { await refreshEnvironments() }

async function onRenameEnvironment() {
  if (!store.organization?.id || !store.project?.id || !store.environment || !environmentName.value.trim()) return

  renamingEnvironment.value = true
  try {
    const updated = await cplaneFetch(
      `/api/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments/${store.environment.id as ':environment_id'}` as const,
      { method: 'PATCH', body: { name: environmentName.value.trim() } },
    )
    syncEnvironment(store, updated)
    renameEnvironmentModal.value = false
    toast.add({ title: 'Environment renamed', color: 'success' })
  } catch (error) {
    toast.add({ title: (error as { data?: { message?: string } })?.data?.message || 'Failed to rename environment', color: 'error' })
  } finally {
    renamingEnvironment.value = false
  }
}

async function onConfirmDeleteEnvironment() {
  if (!store.organization?.id || !store.project?.id || !store.environment) return;
  try {
    await cplaneFetch(`/api/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments/${store.environment.id as ':environment_id'}` as const, { method: 'DELETE' });
    toast.add({ title: 'Environment removed', color: 'success' });
    deleteEnvironmentModal.value = false;
    store.environment = null;
    await refreshEnvironments();

    const list = store.environments
    const target = list.find(b => b.id === routeEnvironmentId.value) ?? list.find(b => b.is_default) ?? list[0] ?? null
    if (target) {
      store.environment = target
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

</script>

<template>
  <div class="min-w-0" :class="props.collapsed ? 'flex flex-col items-center gap-2' : 'space-y-2'">
    <UDropdownMenu size="sm" :items="projectMenuItems" :content="{ align: 'start', collisionPadding: 12 }" :ui="{ content: 'w-(--reka-dropdown-menu-trigger-width)' }" class="w-full">
      <UButton
        :label="props.collapsed ? undefined : projectLabel"
        :icon="props.collapsed ? ICONS.folder : undefined"
        :trailing-icon="props.collapsed ? undefined : ICONS.chevronUpDown"
        :aria-label="props.collapsed ? projectLabel : undefined"
        :title="props.collapsed ? projectLabel : undefined"
        :square="props.collapsed"
        block
        size="sm"
        color="neutral"
        variant="soft"
        class="min-h-8 border border-default bg-elevated/70 hover:bg-elevated data-[state=open]:bg-elevated"
        :class="props.collapsed ? 'size-8' : undefined"
        :ui="{ trailingIcon: 'text-dimmed' }"
      />
    </UDropdownMenu>

    <UDropdownMenu size="sm" :items="environmentMenuItems" :content="{ align: 'start', collisionPadding: 12 }" :ui="{ content: 'w-(--reka-dropdown-menu-trigger-width)' }" class="w-full">
      <template #item-trailing="{ item }">
        <div v-if="item.badges?.length" class="ml-auto flex items-center gap-1">
          <UBadge v-for="badge in item.badges" :key="badge" :color="badge === 'Preview' ? 'primary' : 'neutral'" variant="soft" size="sm">
            {{ badge }}
          </UBadge>
        </div>
      </template>
      <UButton
        :label="props.collapsed ? undefined : environmentLabel"
        :icon="props.collapsed ? ICONS.globeAlt : undefined"
        :trailing-icon="props.collapsed ? undefined : ICONS.chevronUpDown"
        :aria-label="props.collapsed ? environmentLabel : undefined"
        :title="props.collapsed ? environmentLabel : undefined"
        :square="props.collapsed"
        :disabled="!routeProjectId && !store.project"
        block
        size="sm"
        color="neutral"
        variant="soft"
        class="min-h-8 border border-default bg-elevated/70 hover:bg-elevated data-[state=open]:bg-elevated"
        :class="props.collapsed ? 'size-8' : undefined"
        :ui="{ trailingIcon: 'text-dimmed' }"
      />
    </UDropdownMenu>

    <template v-if="store.project">
      <UButton
        :label="props.collapsed ? undefined : 'Environment graph'"
        :icon="ICONS.graph"
        :aria-label="props.collapsed ? 'Environment graph' : undefined"
        :title="props.collapsed ? 'Environment graph' : undefined"
        :square="props.collapsed"
        size="sm"
        variant="ghost"
        color="neutral"
        class="shrink-0"
        :class="props.collapsed ? 'size-8' : 'min-h-8 w-full justify-start'"
        @click="graphModalOpen = true"
      />
    </template>

    <DashboardProjectsCreateModal v-model:open="createProjectModal" @created="onProjectCreated" />
    <DashboardProjectsDeleteModal v-model:open="deleteProjectModal" @deleted="onProjectDeleted" />
    <DashboardProjectsCreateEnvironmentModal v-model:open="createEnvironmentModal" @created="onEnvironmentCreated" />
    <DashboardProjectsEnvironmentGraphModal v-model:open="graphModalOpen" />

    <UModal v-model:open="renameEnvironmentModal" title="Rename Environment" :ui="{ content: 'max-w-sm' }">
      <template #body>
        <form class="space-y-4" @submit.prevent="onRenameEnvironment">
          <UFormField label="Environment name" required>
            <UInput v-model="environmentName" :disabled="renamingEnvironment" autofocus class="w-full" />
          </UFormField>
          <div class="flex justify-end gap-3">
            <UButton color="neutral" variant="ghost" :disabled="renamingEnvironment" @click="renameEnvironmentModal = false">Cancel</UButton>
            <UButton type="submit" :icon="ICONS.check" color="primary" :loading="renamingEnvironment" :disabled="!environmentName.trim()">Save</UButton>
          </div>
        </form>
      </template>
    </UModal>

    <UModal v-model:open="deleteEnvironmentModal" title="Delete Environment" :ui="{ content: 'max-w-sm' }">
      <template #body>
        <p class="text-sm">
          Are you sure you want to delete the environment <strong class="capitalize">{{ store.environment?.name }}</strong>? {{ store.environment?.is_preview ? 'Its timeline revisions will be deleted.' : 'Timeline revisions will be preserved and can be repointed to.' }}
        </p>
        <div class="flex justify-end gap-3 pt-4">
          <UButton variant="ghost" color="neutral" @click="deleteEnvironmentModal = false">Cancel</UButton>
          <UButton color="error" :icon="ICONS.trash" @click="onConfirmDeleteEnvironment">Delete</UButton>
        </div>
      </template>
    </UModal>
</div>
</template>
