<script setup lang="ts">
import { loadProjectEnvironments } from '~/utils/auth'
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const router = useRouter()
const selectingProjectId = ref<string>()
const error = ref('')

async function selectProject(projectId: string) {
  if (selectingProjectId.value) return

  selectingProjectId.value = projectId
  error.value = ''

  try {
    await loadProjectEnvironments(projectId)

    const organizationPath = `/${route.params.organization_slug}`
    const currentPath = route.path.replace(/\/+$/, '')
    if (currentPath !== organizationPath) {
      await router.push(`${currentPath}/${projectId}`)
    }
  } catch {
    error.value = 'Could not load this project’s environments. Select it again to retry.'
  } finally {
    selectingProjectId.value = undefined
  }
}
</script>

<template>
  <div class="flex w-full max-w-375 flex-col gap-4 mx-auto">
    <header class="border-b border-default/60 pb-5">
      <p class="font-mono text-[10px] uppercase tracking-[0.08em] text-muted">Workspace / Projects</p>
      <h1 class="mt-2 text-2xl font-semibold">Select a project to continue</h1>
      <p class="mt-1 max-w-xl text-sm leading-relaxed text-muted">The view you’re trying to access is project-scoped. Select a project to load its environments and resources.</p>
    </header>

    <section class="overflow-hidden rounded-lg border border-default/60 bg-default">
      <div class="flex items-center justify-between gap-3 border-b border-default/60 bg-elevated/40 px-4 py-3">
        <h2 class="text-sm font-semibold">Projects</h2>
        <span class="text-xs text-muted">{{ store.projects.length }}</span>
      </div>

      <div v-if="store.projects.length">
        <UButton
          v-for="project in store.projects"
          :key="project.id"
          :icon="ICONS.folder"
          :trailing-icon="ICONS.chevronRight"
          :label="project.name"
          color="neutral"
          variant="soft"
          block
          :loading="selectingProjectId === project.id"
          :disabled="Boolean(selectingProjectId)"
          class="min-h-12 w-full justify-between rounded-none border-b border-default/60 bg-transparent px-4 py-3 text-left last:border-b-0 hover:bg-elevated"
          :ui="{ leadingIcon: 'text-dimmed', trailingIcon: 'text-dimmed' }"
          @click="selectProject(project.id)"
        />
      </div>

      <div v-else class="px-4 py-10 text-center">
        <UIcon :name="ICONS.folder" class="size-8 text-muted" aria-hidden="true" />
        <p class="mt-3 font-medium">No projects available</p>
        <p class="mt-1 text-sm text-muted">Create a project to start working in C-Plane.</p>
      </div>

      <p v-if="error" class="border-t border-error/20 bg-error/5 px-4 py-3 text-sm text-error" role="alert">{{ error }}</p>
    </section>
  </div>
</template>
