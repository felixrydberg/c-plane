<script setup lang="ts">
const store = useStore()
const route = useRoute()

const projectId = computed(() => route.params.project_id?.toString() || null)
const slug = computed(() => route.params.organization_slug?.toString())

// Try store first (populated by dashboard-projects-nav during SSR)
const project = computed(() => store.projects.find(p => p.id === projectId.value))

if (project.value?.default_branch_id && slug.value) {
  await navigateTo(`/${slug.value}/containers/${projectId.value}/${project.value.default_branch_id}`, { replace: true })
} else if (projectId.value && slug.value && store.organization?.id) {
  // Fallback: fetch project from API
  const { data } = await useFetch<{ default_branch_id?: string }>(
    `/api/backend/organization/${store.organization.id}/projects/${projectId.value}`
  )
  if (data.value?.default_branch_id) {
    await navigateTo(`/${slug.value}/containers/${projectId.value}/${data.value.default_branch_id}`, { replace: true })
  }
}
</script>

<template>
  <div class="flex flex-col items-center justify-center py-24 gap-4 text-center">
    <p class="text-sm text-muted">Loading default branch...</p>
  </div>
</template>
