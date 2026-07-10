<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const store = useStore();

const selectedProject = computed(() => store.project);
const showOnboarding = computed(() => store.projects.length === 0);
</script>

<template>
  <DashboardProjectsOnboarding v-if="showOnboarding" />

  <DashboardProjectsSelectPrompt v-else-if="!selectedProject" />

  <div v-else class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div>
      <h1 class="text-2xl font-semibold">{{ selectedProject.name }}</h1>
      <p class="text-muted text-sm mt-1">Project overview and activity.</p>
    </div>

    <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
      <UCard>
        <div class="flex items-center gap-3">
          <div class="flex items-center justify-center size-10 rounded-lg bg-primary/10">
            <UIcon :name="ICONS.databases" class="size-5 text-primary" />
          </div>
          <div>
            <p class="text-sm text-muted">Databases</p>
            <p class="text-2xl font-semibold">0</p>
          </div>
        </div>
      </UCard>
      <UCard>
        <div class="flex items-center gap-3">
          <div class="flex items-center justify-center size-10 rounded-lg bg-primary/10">
            <UIcon :name="ICONS.containers" class="size-5 text-primary" />
          </div>
          <div>
            <p class="text-sm text-muted">Containers</p>
            <p class="text-2xl font-semibold">0</p>
          </div>
        </div>
      </UCard>
      <UCard>
        <div class="flex items-center gap-3">
          <div class="flex items-center justify-center size-10 rounded-lg bg-primary/10">
            <UIcon :name="ICONS.secrets" class="size-5 text-primary" />
          </div>
          <div>
            <p class="text-sm text-muted">Secrets</p>
            <p class="text-2xl font-semibold">0</p>
          </div>
        </div>
      </UCard>
    </div>
  </div>
</template>
