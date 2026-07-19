<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const store = useStore();

const selectedProject = computed(() => store.project);
const showOnboarding = computed(() => store.projects.length === 0);
const currentEnvironment = computed(() =>
  store.environment?.name
  ?? store.environments.find(environment => environment.id === selectedProject.value?.default_environment_id)?.name
  ?? 'No environment selected',
)
</script>

<template>
  <DashboardProjectsOnboarding v-if="showOnboarding" />

  <DashboardProjectsSelectPrompt v-else-if="!selectedProject" />

  <div v-else class="mx-auto flex w-full max-w-6xl flex-col gap-8">
    <div class="flex flex-col gap-5 border-b border-default pb-6 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <p class="font-space-mono text-[11px] uppercase tracking-[0.08em] text-muted">Project overview</p>
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
              <p class="text-sm text-muted">Databases</p>
              <p class="mt-3 text-4xl font-normal tracking-[-0.04em]">0</p>
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
              <p class="mt-3 text-4xl font-normal tracking-[-0.04em]">0</p>
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
