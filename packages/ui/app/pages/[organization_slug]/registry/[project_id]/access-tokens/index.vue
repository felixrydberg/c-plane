<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const organizationId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() ?? '')
const project = computed(() => store.projects.find(item => item.id === projectId.value))
const isOwner = computed(() => store.organization?.member?.role === 'owner')
</script>

<template>
  <div class="flex w-full max-w-375 flex-col gap-5 mx-auto">
    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <UiBackLink label="Registry" :to="`/${route.params.organization_slug}/registry/${projectId}`" />
        <UiPageEyebrow label="Storage &amp; Databases" />
        <h1 class="mt-2 text-2xl font-semibold">Access Tokens</h1>
        <p class="mt-1 text-sm text-muted">Credentials for container clients and CI pipelines in {{ project?.name ?? projectId }}.</p>
      </div>
      <UButton v-if="isOwner" :icon="ICONS.plus" color="primary" :to="`/${route.params.organization_slug}/registry/${projectId}/access-tokens/new`">New Access Token</UButton>
    </div>
    <RegistryAccessTokensPanel v-if="organizationId && projectId" :organization-id="organizationId" :project-id="projectId" />
  </div>
</template>
