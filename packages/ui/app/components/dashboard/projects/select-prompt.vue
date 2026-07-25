<script setup lang="ts">
import { loadProjectEnvironments } from '~/utils/auth'
import { ICONS } from '~/utils/icons'

const store = useStore()
const selectingProjectId = ref<string>()
const error = ref('')

async function selectProject(projectId: string) {
  if (selectingProjectId.value) return

  selectingProjectId.value = projectId
  error.value = ''

  try {
    await loadProjectEnvironments(projectId)
  } catch {
    error.value = 'Could not load this project’s environments. Select it again to retry.'
  } finally {
    selectingProjectId.value = undefined
  }
}
</script>

<template>
  <div class="project-selection mx-auto flex min-h-0 w-full max-w-4xl items-start justify-center px-6 py-4 pb-10">
    <section class="project-selection__panel w-full overflow-hidden rounded-md border border-dashed border-default bg-transparent">
      <header class="project-selection__header border-b border-default px-6 py-6 sm:px-8">
        <p class="font-space-mono text-[11px] uppercase tracking-[0.08em] text-primary">Workspace / Projects</p>
        <h1 class="mt-3 text-3xl font-normal tracking-[-0.04em] sm:text-4xl">Select a project</h1>
        <p class="mt-3 max-w-xl text-sm leading-relaxed text-muted">
          Choose the project you want to work in, then C-Plane will load its environments and resources.
        </p>
      </header>

      <div class="project-selection__projects px-6 py-6 sm:px-8">
        <p class="font-space-mono text-[10px] uppercase tracking-[0.08em] text-muted">Quick select</p>
        <div class="mt-3 grid gap-2 sm:grid-cols-2">
          <UButton
            v-for="project in store.projects"
            :key="project.id"
            :icon="ICONS.folder"
            :trailing-icon="ICONS.arrowTopRight"
            color="neutral"
            variant="solid"
            block
            :loading="selectingProjectId === project.id"
            :disabled="Boolean(selectingProjectId)"
            class="justify-start"
            @click="selectProject(project.id)"
          >
            {{ project.name }}
          </UButton>
        </div>
        <p v-if="error" class="mt-3 text-sm text-error" role="alert">{{ error }}</p>
      </div>
    </section>
  </div>
</template>

<style scoped>
.project-selection__panel {
  animation: project-selection-enter 480ms cubic-bezier(0.23, 1, 0.32, 1) both;
}

.project-selection__header,
.project-selection__projects {
  animation: project-selection-rise 420ms cubic-bezier(0.23, 1, 0.32, 1) both;
}

.project-selection__header { animation-delay: 80ms; }
.project-selection__projects { animation-delay: 150ms; }

@keyframes project-selection-enter {
  from { opacity: 0; transform: scale(0.985); }
  to { opacity: 1; transform: scale(1); }
}

@keyframes project-selection-rise {
  from { opacity: 0; transform: translateY(12px); }
  to { opacity: 1; transform: translateY(0); }
}

@media (prefers-reduced-motion: reduce) {
  .project-selection__panel,
  .project-selection__header,
  .project-selection__projects {
    animation: none;
  }
}
</style>
