<script setup lang="ts">
definePageMeta({
  redirect: (to) => {
    const store = useStore()
    const project = store.projects.find(p => p.id === to.params.project_id)
    if (project?.default_branch_id) {
      return `/${to.params.organization_slug}/containers/${to.params.project_id}/${project.default_branch_id}`
    }
  },
})
</script>

<template>
  <DashboardProjectsSelectPrompt />
</template>
