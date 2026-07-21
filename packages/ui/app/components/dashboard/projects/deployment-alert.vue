<script setup lang="ts">
const store = useStore()
const toast = useToast()
const deploying = ref(false)

const hasRecentUndeployedRevision = computed(() =>
  store.environments.find(environment => environment.id === store.environment?.id)?.has_recent_undeployed_revision ?? false
)

async function deployLatestRevision() {
  if (!store.organization?.id || !store.project?.id || !store.environment) return
  deploying.value = true
  try {
    const revisions = await $fetch(
      `/api/cplane/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/timelines` as const,
      { query: { environment_id: store.environment.id } }
    )
    const latest = revisions[0]
    if (!latest) return

    await $fetch(
      `/api/cplane/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments/${store.environment.id as ':environment_id'}` as const,
      { method: 'PATCH', body: { timeline_id: latest.id } }
    )
    store.environment.has_recent_undeployed_revision = false
    const environment = store.environments.find(environment => environment.id === store.environment?.id)
    if (environment) environment.has_recent_undeployed_revision = false
    store.refreshKey++
    toast.add({ title: 'Latest revision deployed', color: 'success' })
  } catch {
    toast.add({ title: 'Failed to deploy latest revision', color: 'error' })
  } finally {
    deploying.value = false
  }
}
</script>

<template>
  <div v-if="hasRecentUndeployedRevision" class="shrink-0 bg-default px-6 py-3 lg:px-8">
    <UAlert color="warning" variant="subtle" icon="i-heroicons:exclamation-triangle" title="Environment updated, not deployed" description="Deploy the latest revision to make its changes active." orientation="horizontal" class="border border-warning/40 bg-warning/15 text-warning-800 dark:border-warning-400/40 dark:bg-warning-950/40 dark:text-warning-200">
      <template #actions>
        <UButton color="primary" size="sm" :loading="deploying" @click="deployLatestRevision">Deploy latest</UButton>
      </template>
    </UAlert>
  </div>
</template>
