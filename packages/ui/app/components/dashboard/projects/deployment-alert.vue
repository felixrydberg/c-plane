<script setup lang="ts">
import { ICONS } from '~/utils/icons'
import { syncEnvironment } from '~/utils/environments'

const store = useStore()
const route = useRoute()
const router = useRouter()
const toast = useToast()
const deploying = ref(false)
const reverting = ref(false)

const hasPendingDeployment = computed(() =>
  store.environment?.draft_timeline !== store.environment?.deployed_timeline
)

async function deployDraft() {
  if (!store.organization?.id || !store.project?.id || !store.environment) return
  deploying.value = true
  try {
    const updated = await cplaneFetch(
      `/api/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments/${store.environment.id as ':environment_id'}` as const,
      { method: 'PATCH', body: { deployed_timeline_id: store.environment.draft_timeline } }
    )
    syncEnvironment(store, updated)
    store.refreshKey++
    toast.add({ title: 'Draft revision deployed', color: 'success' })
  } catch {
    toast.add({ title: 'Failed to deploy draft revision', color: 'error' })
  } finally {
    deploying.value = false
  }
}

async function revertDraft() {
  if (!store.organization?.id || !store.project?.id || !store.environment) return
  reverting.value = true
  try {
    const updated = await cplaneFetch(
      `/api/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments/${store.environment.id as ':environment_id'}` as const,
      { method: 'PATCH', body: { draft_timeline_id: store.environment.deployed_timeline } }
    )
    syncEnvironment(store, updated)
    const { revision, ...query } = route.query
    await router.replace({ query })
    store.refreshKey++
    toast.add({ title: 'Draft revision reverted', color: 'success' })
  } catch {
    toast.add({ title: 'Failed to revert draft revision', color: 'error' })
  } finally {
    reverting.value = false
  }
}
</script>

<template>
  <div v-if="hasPendingDeployment" class="shrink-0 bg-default px-6 py-3 lg:px-8">
    <UAlert color="warning" variant="subtle" icon="i-heroicons:exclamation-triangle" title="Draft revision pending deployment" description="Deploy the draft revision to make it live." orientation="horizontal" class="border border-warning/40 bg-warning/15 text-warning-800 dark:border-warning-400/40 dark:bg-warning-950/40 dark:text-warning-200">
      <template #actions>
        <UButton :icon="ICONS.check" color="primary" size="sm" :loading="deploying" @click="deployDraft">Deploy draft revision</UButton>
        <UButton :icon="ICONS.refresh" color="neutral" variant="solid" size="sm" :loading="reverting" @click="revertDraft">Revert draft to deployed version</UButton>
      </template>
    </UAlert>
  </div>
</template>
