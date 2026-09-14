<script setup lang="ts">
import type { Container, DatabaseWithBranches } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'

const store = useStore();

const selectedProject = computed(() => store.project);
const organizationId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => selectedProject.value?.id ?? '')
const currentEnvironment = computed(() =>
  store.environment?.name
  ?? store.environments.find(environment => environment.id === selectedProject.value?.default_environment_id)?.name
  ?? 'No environment selected',
)

const containersUrl = computed(() => organizationId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/containers` as const
  : '')
const databasesUrl = computed(() => organizationId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/databases/postgres` as const
  : '')
const resourcesReady = computed(() => Boolean(organizationId.value && projectId.value))

const { data: containers } = await useCplaneFetch<Container[]>(containersUrl, {
  default: () => [],
  immediate: resourcesReady,
  query: { project_id: projectId },
})
const { data: databases } = await useCplaneFetch<DatabaseWithBranches[]>(databasesUrl, {
  default: () => [],
  immediate: resourcesReady,
  query: { project_id: projectId },
})
</script>

<template>
  <DashboardProjectsSelectPrompt v-if="!selectedProject" />

  <div v-else class="mx-auto flex w-full max-w-6xl flex-col gap-8">
    <div class="flex flex-col gap-5 border-b border-default pb-6 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <p class="font-space-mono text-[11px] uppercase tracking-[0.08em] text-muted">Project overview</p>
        <UiPageEyebrow label="Overview" />
        <h1 class="mt-2 text-4xl font-normal tracking-[-0.04em]">{{ selectedProject.name }}</h1>
        <p class="mt-2 text-sm text-muted">Infrastructure, deployments, and credentials in one place.</p>
      </div>

      <div class="rounded-md border border-dashed border-default bg-transparent px-4 py-3 sm:min-w-52">
        <p class="font-space-mono text-[10px] uppercase tracking-[0.08em] text-muted">Current environment</p>
        <p class="mt-1 flex items-center gap-2 text-sm font-medium">
          <span class="size-1.5 rounded-full bg-primary" aria-hidden="true" />
          {{ currentEnvironment }}
        </p>
      </div>
    </div>

    <section>
      <div class="mb-4 flex items-center justify-between">
        <h2 class="text-lg font-normal tracking-[-0.02em]">Resources</h2>
        <p class="font-space-mono text-[10px] uppercase tracking-[0.08em] text-muted">Current project</p>
      </div>

      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <UCard class="border-dashed border-default bg-transparent" :ui="{ body: 'p-5' }">
          <div class="flex items-start justify-between gap-4">
            <div>
              <p class="text-sm text-muted">Postgres</p>
              <p class="mt-3 text-4xl font-normal tracking-[-0.04em]">{{ databases?.length ?? 0 }}</p>
            </div>
            <div class="flex size-10 items-center justify-center rounded-md bg-primary/10">
              <UIcon :name="ICONS.databases" class="size-5 text-primary" />
            </div>
          </div>
        </UCard>
        <UCard class="border-dashed border-default bg-transparent" :ui="{ body: 'p-5' }">
          <div class="flex items-start justify-between gap-4">
            <div>
              <p class="text-sm text-muted">Containers</p>
              <p class="mt-3 text-4xl font-normal tracking-[-0.04em]">{{ containers?.length ?? 0 }}</p>
            </div>
            <div class="flex size-10 items-center justify-center rounded-md bg-primary/10">
              <UIcon :name="ICONS.containers" class="size-5 text-primary" />
            </div>
          </div>
        </UCard>
      </div>
    </section>
  </div>
</template>
