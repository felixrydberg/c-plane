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
  !route.query.revision || route.query.revision === store.environment?.deployed_timeline
)
const hasDraftRevision = computed(() =>
  !!store.environment && store.environment.draft_timeline !== store.environment.deployed_timeline
)

// Projects are loaded by the auth plugin on every request.
// Refresh only needed after create/delete.
async function refreshProjects() {
  if (!store.organization?.id) return
  const { data } = await $fetch(`/api/cplane/organization/${store.organization.id as ':organization_id'}/projects` as const)
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
const revisionContainer = ref<HTMLElement>()

const projectLabel = computed(() => store.project?.name || 'All Projects')

const environmentLabel = computed(() => {
  if (!store.project) return 'Select environment'
  if (!store.environment) return store.environments.length ? 'Select environment' : 'No environments'
  return store.environment.name
})

const projectMenuItems = computed<DropdownMenuItem[][]>(() => {
  return [[
    { label: 'All Projects', onSelect() { void selectProject(null) } },
    ...store.projects.map(project => ({
      label: project.name,
      onSelect() { void selectProject(project.id) },
    })),
  ], [
  { label: 'Create Project', onSelect() { createProjectModal.value = true } },
  ...(store.project ? [{ label: 'Delete Project', color: 'error' as const, onSelect() { deleteProjectModal.value = true } }] : []),
  ]]
})

const environmentMenuItems = computed<DropdownMenuItem[][]>(() => {
  return [
    store.environments.map(environment => ({
      label: environment.name,
      badges: [
        ...(environment.is_default ? ['Default'] : []),
        ...(environment.is_preview ? ['Preview'] : []),
      ],
      onSelect() { selectEnvironment(environment) },
    })),
    [
      { label: 'Create Environment', onSelect() { createEnvironmentModal.value = true } },
      ...(store.environment ? [{
        label: 'Rename Environment',
        onSelect() {
          environmentName.value = store.environment?.name ?? ''
          renameEnvironmentModal.value = true
        },
      }] : []),
      ...(store.environment ? [{
        label: 'Delete Environment',
        color: 'error' as const,
        disabled: store.environment.is_default,
        onSelect() {
          if (!store.environment?.is_default) deleteEnvironmentModal.value = true
        },
      }] : []),
    ],
  ]
})

const revisionItems = computed<DropdownMenuItem[][]>(() => [[
  {
    label: 'Draft',
    icon: ICONS.pencil,
    disabled: !hasDraftRevision.value,
    onSelect() { setRevisionView(false) },
  },
  {
    label: 'Deployed',
    icon: ICONS.check,
    onSelect() { setRevisionView(true) },
  },
]])

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
  const wasViewingDeployed = isViewingDeployed.value
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
  router.push(`${url}${wasViewingDeployed ? '' : `?revision=${b.draft_timeline}`}`)
}

function setRevisionView(viewingDeployed: boolean) {
  if (!store.environment || !hasDraftRevision.value) return
  const query = { ...route.query }
  if (viewingDeployed) delete query.revision
  else query.revision = store.environment.draft_timeline
  router.push({ query })
}

function setRevisionHeight(height?: number) {
  if (revisionContainer.value) revisionContainer.value.style.height = height === undefined ? '' : `${height}px`
}

function beforeRevisionEnter() {
  setRevisionHeight(0)
}

function enterRevision(element: Element) {
  if (!(element instanceof HTMLElement)) return
  requestAnimationFrame(() => setRevisionHeight(element.offsetHeight))
}

function beforeRevisionLeave(element: Element) {
  if (element instanceof HTMLElement) setRevisionHeight(element.offsetHeight)
}

function leaveRevision() {
  requestAnimationFrame(() => setRevisionHeight(0))
}

function afterRevisionEnter() {
  setRevisionHeight()
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
    const updated = await $fetch(
      `/api/cplane/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments/${store.environment.id as ':environment_id'}` as const,
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
    await $fetch(`/api/cplane/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments/${store.environment.id as ':environment_id'}` as const, { method: 'DELETE' });
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

const graphModalOpen = ref(false)
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
      <div ref="revisionContainer" class="revision-height">
        <Transition
          name="revision"
          @before-enter="beforeRevisionEnter"
          @enter="enterRevision"
          @after-enter="afterRevisionEnter"
          @before-leave="beforeRevisionLeave"
          @leave="leaveRevision"
        >
          <div v-if="store.environment && !props.collapsed" class="space-y-1.5 pt-1">
            <p class="px-1 text-[10px] font-mono uppercase tracking-[0.08em] text-muted">Revision</p>
            <div class="grid grid-cols-2 gap-1 rounded-md border border-default bg-elevated/70 p-1">
              <UButton
                size="xs"
                :color="isViewingDeployed ? 'neutral' : 'primary'"
                :variant="isViewingDeployed ? 'ghost' : 'soft'"
                :disabled="!hasDraftRevision"
                class="w-full justify-center"
                @click="setRevisionView(false)"
              >
                Draft
              </UButton>
              <UButton
                size="xs"
                :color="isViewingDeployed ? 'primary' : 'neutral'"
                :variant="isViewingDeployed ? 'soft' : 'ghost'"
                class="w-full justify-center"
                @click="setRevisionView(true)"
              >
                Deployed
              </UButton>
            </div>
          </div>
        </Transition>
      </div>

      <UDropdownMenu v-if="props.collapsed" size="sm" :items="revisionItems" :content="{ align: 'start', collisionPadding: 12 }" :ui="{ content: 'w-40' }">
        <UButton
          :icon="ICONS.revision"
          :aria-label="isViewingDeployed ? 'Deployed revision' : 'Draft revision'"
          :title="isViewingDeployed ? 'Deployed revision' : 'Draft revision'"
          size="sm"
          color="neutral"
          variant="soft"
          square
          class="size-8 border border-default bg-elevated/70 hover:bg-elevated data-[state=open]:bg-elevated"
        />
      </UDropdownMenu>

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

<style scoped>
.revision-height {
  overflow: hidden;
  transition: height 180ms cubic-bezier(0.23, 1, 0.32, 1);
}

.revision-enter-active,
.revision-leave-active {
  will-change: opacity, transform;
  transition-property: opacity, transform;
  transition-timing-function: cubic-bezier(0.23, 1, 0.32, 1);
}

.revision-enter-active {
  transition-duration: 180ms;
}

.revision-leave-active {
  transition-duration: 120ms;
}

.revision-enter-from,
.revision-leave-to {
  opacity: 0;
  transform: translateY(-6px) scaleY(0.96);
  transform-origin: top;
}

@media (prefers-reduced-motion: reduce) {
  .revision-height {
    transition-duration: 120ms;
  }

  .revision-enter-active,
  .revision-leave-active {
    transition-duration: 120ms;
    transition-property: opacity;
  }

  .revision-enter-from,
  .revision-leave-to {
    transform: none;
  }
}
</style>
