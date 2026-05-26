<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const store = useStore();
const route = useRoute();

const projectId = computed(() => route.query.project?.toString() || null);

useHead({
  title: `Storage - ${projectId.value ? store.projects.find(p => p.id === projectId.value)?.name : 'All Projects'} - C-Plane`,
});
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-semibold">Storage</h1>
        <p class="text-muted text-sm mt-1">
          <template v-if="projectId && store.projects.find(p => p.id === projectId)">Manage S3-compatible storage buckets for {{ store.projects.find(p => p.id === projectId)?.name }}.</template>
          <template v-else>Manage S3-compatible storage buckets across all projects.</template>
        </p>
      </div>
      <UButton :icon="ICONS.plus">New Bucket</UButton>
    </div>

    <div class="flex flex-col items-center justify-center py-16 gap-3 text-center border border-dashed border-default rounded-lg">
      <UIcon :name="ICONS.storage" class="size-10 text-muted" />
      <p class="text-muted" v-if="projectId">No storage buckets yet. Create one to get started.</p>
      <p class="text-muted" v-else>Select a project to manage its storage buckets.</p>
    </div>
  </div>
</template>
