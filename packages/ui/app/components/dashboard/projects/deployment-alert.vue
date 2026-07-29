<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const store = useStore()
const toast = useToast()
const deploying = ref(false)

const hasPendingDeployment = computed(() =>
  store.environment?.draft_timeline !== store.environment?.deployed_timeline
)

async function deployDraft() {
  if (!store.organization?.id || !store.project?.id || !store.environment) return
  deploying.value = true
  try {
    const updated = await $fetch(
      `/api/cplane/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments/${store.environment.id as ':environment_id'}` as const,
      { method: 'PATCH', body: { deployed_timeline_id: store.environment.draft_timeline } }
    )
    store.environment = updated
    const index = store.environments.findIndex(environment => environment.id === updated.id)
    if (index !== -1) store.environments[index] = updated
    store.refreshKey++
    toast.add({ title: 'Draft revision deployed', color: 'success' })
  } catch {
    toast.add({ title: 'Failed to deploy draft revision', color: 'error' })
  } finally {
    deploying.value = false
  }
}
</script>

<template>
  <div v-if="hasPendingDeployment" class="shrink-0 bg-default px-6 py-3 lg:px-8">
    <UAlert color="warning" variant="subtle" icon="i-heroicons:exclamation-triangle" title="Draft revision pending deployment" description="Deploy the draft revision to make it live." orientation="horizontal" class="border border-warning/40 bg-warning/15 text-warning-800 dark:border-warning-400/40 dark:bg-warning-950/40 dark:text-warning-200">
      <template #actions>
        <UButton :icon="ICONS.check" color="primary" size="sm" :loading="deploying" @click="deployDraft">Deploy draft revision</UButton>
      </template>
    </UAlert>
  </div>
</template>
