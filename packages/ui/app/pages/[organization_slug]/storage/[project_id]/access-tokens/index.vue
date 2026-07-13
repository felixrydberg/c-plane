<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const organizationId = computed(() => store.organization?.id || '')
const projectId = computed(() => route.params.project_id as string)
</script>

<template>
  <div class="w-full max-w-[1500px] mx-auto">
    <div class="overflow-hidden rounded-lg border border-default/60 bg-default">
      <header class="flex flex-col gap-4 border-b border-default/60 px-5 py-4 sm:flex-row sm:items-center sm:justify-between">
        <div><UiBackLink label="Buckets" :to="`/${route.params.organization_slug}/storage/${projectId}`" /><h1 class="mt-2 text-xl font-semibold">Access Tokens</h1><p class="mt-1 text-xs text-muted">Credentials for S3 clients targeting this project.</p></div>
        <UButton :icon="ICONS.plus" color="primary" :to="`/${route.params.organization_slug}/storage/${projectId}/access-tokens/new`">New Access Token</UButton>
      </header>
      <div class="p-5">
        <StorageAccessTokensPanel v-if="organizationId" :organization-id="organizationId" :project-id="projectId" />
      </div>
    </div>
  </div>
</template>
