<script setup lang="ts">
type TimelineRevision = { id: string }

const store = useStore()
const toast = useToast()
const deploying = ref(false)

const hasRecentUndeployedRevision = computed(() =>
  store.branches.find(branch => branch.id === store.branch?.id)?.has_recent_undeployed_revision ?? false
)

async function deployLatestRevision() {
  if (!store.organization?.id || !store.project?.id || !store.branch) return
  deploying.value = true
  try {
    const revisions = await $fetch<TimelineRevision[]>(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/timelines`,
      { query: { branch_id: store.branch.id } }
    )
    const latest = revisions[0]
    if (!latest) return

    await $fetch(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/branches/${store.branch.id}`,
      { method: 'PATCH', body: { timeline_id: latest.id } }
    )
    store.branch.has_recent_undeployed_revision = false
    const branch = store.branches.find(branch => branch.id === store.branch?.id)
    if (branch) branch.has_recent_undeployed_revision = false
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
    <UAlert color="warning" variant="soft" icon="i-heroicons:exclamation-triangle" title="Environment updated, not deployed" description="Deploy the latest revision to make its changes active." orientation="horizontal">
      <template #actions>
        <UButton color="primary" size="sm" :loading="deploying" @click="deployLatestRevision">Deploy latest</UButton>
      </template>
    </UAlert>
  </div>
</template>
