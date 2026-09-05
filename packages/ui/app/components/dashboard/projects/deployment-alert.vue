<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()

const projectId = computed(() => route.params.project_id?.toString() || '')
const environmentId = computed(() => route.params.environment_id?.toString() || '')
const isComputePage = computed(() => {
  const slug = store.organization?.slug
  if (!slug) return false
  const pathAfterSlug = route.path.slice(slug.length + 1)
  return pathAfterSlug === 'compute/containers' || pathAfterSlug.startsWith('compute/containers/')
})
const environmentsUrl = computed(() => store.organization?.id && projectId.value && isComputePage.value
  ? `/api/organization/${store.organization.id as ':organization_id'}/projects/${projectId.value as ':project_id'}/environments` as const
  : '')
const { data: environmentList } = await useCplaneFetch(environmentsUrl, {
  default: () => [],
  immediate: computed(() => !!environmentsUrl.value),
})
const currentEnvironment = computed(() => (Array.isArray(environmentList.value) ? environmentList.value : []).find(item => item.id === environmentId.value)
  ?? (store.environment?.id === environmentId.value ? store.environment : null))
const hasPendingDeployment = computed(() =>
  !!currentEnvironment.value && currentEnvironment.value.draft_timeline !== currentEnvironment.value.deployed_timeline
)
const reviewUrl = computed(() => store.organization?.slug && store.project?.id && currentEnvironment.value?.id
  ? `/${store.organization.slug}/compute/containers/${store.project.id}/${currentEnvironment.value.id}`
  : '')
const isReviewPage = computed(() => !!reviewUrl.value && route.path === reviewUrl.value)
</script>

<template>
  <div v-if="hasPendingDeployment && isComputePage && !isReviewPage" class="shrink-0 bg-default px-6 py-3 lg:px-8">
    <UAlert color="warning" variant="subtle" icon="i-heroicons:exclamation-triangle" :title="`Unpublished changes in ${currentEnvironment?.name}`" description="Review these changes before making them live." orientation="horizontal" class="border border-warning/40 bg-warning/15 text-warning-800 dark:border-warning-400/40 dark:bg-warning-950/40 dark:text-warning-200">
      <template #actions>
        <UButton :icon="ICONS.revision" color="primary" size="sm" :to="reviewUrl">Review changes</UButton>
      </template>
    </UAlert>
  </div>
</template>
