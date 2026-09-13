<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const route = useRoute()
const organizationSlug = computed(() => route.params.organization_slug?.toString() || '')
const projectId = computed(() => typeof route.query.project_id === 'string' ? route.query.project_id : '')
const environmentId = computed(() => typeof route.query.environment_id === 'string' ? route.query.environment_id : '')
const hasEnvironmentScope = computed(() => !!projectId.value && !!environmentId.value)
const environmentUrl = computed(() => hasEnvironmentScope.value
  ? `/${organizationSlug.value}/compute/containers/${projectId.value}/${environmentId.value}`
  : '')
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <UiBackLink v-if="hasEnvironmentScope" label="environment" :to="environmentUrl" />
    <div>
      <UiPageEyebrow :label="hasEnvironmentScope ? 'Environment metrics' : 'Observe'" />
      <h1 class="mt-2 text-2xl font-semibold">{{ hasEnvironmentScope ? 'Environment metrics' : 'Analytics' }}</h1>
      <p class="text-muted text-sm mt-1">{{ hasEnvironmentScope ? 'Monitor performance and usage for this environment.' : 'Monitor performance, usage, and trends across your organization.' }}</p>
    </div>

    <div class="flex flex-col items-center justify-center py-16 gap-3 text-center border border-dashed border-default rounded-lg">
      <UIcon :name="ICONS.analytics" class="size-10 text-muted" />
      <p class="text-muted">{{ hasEnvironmentScope ? 'Environment metrics will appear here once telemetry is available.' : 'Analytics data will appear here once you deploy services.' }}</p>
    </div>
  </div>
</template>
