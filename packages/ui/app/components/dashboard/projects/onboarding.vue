<script setup lang="ts">
import type { Project } from '@cplane/sdk'
import { loadProjectEnvironments } from '~/utils/auth'
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const router = useRouter()
const name = ref('')
const loading = ref(false)
const error = ref('')
const createdProject = ref<Project>()

async function loadCreatedProject() {
  if (!createdProject.value) return

  try {
    await loadProjectEnvironments(createdProject.value.id)
    createdProject.value = undefined
    await router.push(`/${route.params.organization_slug}`)
  } catch {
    error.value = 'Project created, but its environments could not be loaded. Retry loading them.'
  }
}

async function createProject() {
  if (!name.value.trim() || !store.organization?.id || loading.value || createdProject.value) return

  loading.value = true
  error.value = ''

  try {
    const project = await $fetch<Project>(`/api/cplane/organization/${store.organization.id as ':organization_id'}/projects` as const, {
      method: 'POST',
      body: { name: name.value.trim() },
    })

    store.projects = [...store.projects, project]
    createdProject.value = project
    await loadCreatedProject()
  } catch (cause: unknown) {
    const errorCause = cause as { data?: { message?: string }, message?: string }
    error.value = errorCause.data?.message ?? errorCause.message ?? 'Failed to create project'
  } finally {
    loading.value = false
  }
}

async function retryLoadProjectEnvironments() {
  if (loading.value || !createdProject.value) return

  loading.value = true
  error.value = ''
  try {
    await loadCreatedProject()
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="organization-onboarding mx-auto flex min-h-dvh w-full max-w-4xl items-center justify-center px-6 py-10">
    <section class="organization-onboarding__panel w-full overflow-hidden rounded-md border border-dashed border-default bg-transparent">
      <header class="organization-onboarding__header border-b border-default px-6 py-6 sm:px-8">
        <p class="font-space-mono text-[11px] uppercase tracking-[0.08em] text-primary">
          {{ store.organization?.name }} / Setup
        </p>
        <h1 class="mt-3 text-3xl font-normal tracking-[-0.04em] sm:text-4xl">Create your first project</h1>
        <p class="mt-3 max-w-xl text-sm leading-relaxed text-muted">
            Keep the databases, containers, and secrets that ship together in one place.
        </p>
      </header>

      <form class="organization-onboarding__form grid gap-4 px-6 py-6 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end sm:px-8" @submit.prevent="createProject">
        <UFormField label="Project name" required class="w-full">
          <div class="flex gap-3">
            <UInput
              v-model="name"
              placeholder="my-awesome-project"
              :disabled="loading"
              autofocus
              class="w-full"
            />
            <UButton class="whitespace-nowrap" type="submit" :icon="ICONS.plus" :loading="loading" :disabled="!name.trim() || Boolean(createdProject)">
              Create project
            </UButton>
          </div>
          <p class="pt-2 text-xs text-muted">Choose a name that describes what this project does.</p>
        </UFormField>
        <div v-if="error" class="flex items-center gap-3 text-sm text-error sm:col-span-2" role="alert">
          <span>{{ error }}</span>
          <UButton v-if="createdProject" type="button" :icon="ICONS.refresh" color="error" size="sm" :loading="loading" @click="retryLoadProjectEnvironments">Retry</UButton>
        </div>
      </form>

    </section>
  </div>
</template>

<style scoped>
.organization-onboarding__panel {
  animation: organization-onboarding-enter 480ms cubic-bezier(0.23, 1, 0.32, 1) both;
}

.organization-onboarding__header,
.organization-onboarding__form {
  animation: organization-onboarding-rise 420ms cubic-bezier(0.23, 1, 0.32, 1) both;
}

.organization-onboarding__header { animation-delay: 80ms; }
.organization-onboarding__form { animation-delay: 150ms; }

@keyframes organization-onboarding-enter {
  from { opacity: 0; transform: scale(0.985); }
  to { opacity: 1; transform: scale(1); }
}

@keyframes organization-onboarding-rise {
  from { opacity: 0; transform: translateY(12px); }
  to { opacity: 1; transform: translateY(0); }
}

@media (prefers-reduced-motion: reduce) {
  .organization-onboarding__panel,
  .organization-onboarding__header,
  .organization-onboarding__form {
    animation: none;
  }
}
</style>
