<script setup lang="ts">
const route = useRoute()

const projectId = computed(() => route.params.project_id?.toString())

if (import.meta.server && projectId.value) {
  // This case is handled by Nuxt's route matching — [project_id]/index.vue
}
</script>

<template>
  <DashboardProjectsSelectPrompt />
</template>
