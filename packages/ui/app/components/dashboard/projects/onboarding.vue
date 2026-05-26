<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const createModalOpen = ref(false);
</script>

<template>
  <div class="flex flex-1 items-center justify-center">
    <div class="flex flex-col items-center gap-6 text-center max-w-md">
      <div class="flex items-center justify-center size-16 rounded-xl bg-primary/10">
        <UIcon :name="ICONS.folderPlus" class="size-8 text-primary" />
      </div>
      <div class="flex flex-col gap-2">
        <h1 class="text-2xl font-semibold">Create your first project</h1>
        <p class="text-muted text-sm leading-relaxed">
          Projects let you organize databases, containers, and secrets together.
          Create one to get started.
        </p>
      </div>
      <UButton size="lg" @click="createModalOpen = true">
        New Project
      </UButton>

      <DashboardProjectsCreateModal v-model:open="createModalOpen" />
    </div>
  </div>
</template>
