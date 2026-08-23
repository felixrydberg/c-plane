<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const organizationId = computed(() => store.organization?.id ?? '')
const isOwner = computed(() => store.organization?.member?.role === 'owner')
</script>

<template>
  <div class="w-full max-w-[1500px] mx-auto">
    <div class="overflow-hidden rounded-lg border border-default/60 bg-default">
      <header class="flex flex-col gap-4 border-b border-default/60 px-5 py-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <UiBackLink label="Registry" :to="`/${route.params.organization_slug}/registry`" />
          <h1 class="mt-2 text-xl font-semibold">Access Tokens</h1>
          <p class="mt-1 text-xs text-muted">Credentials for container clients and CI pipelines in this organization.</p>
        </div>
        <UButton v-if="isOwner" :icon="ICONS.plus" color="primary" :to="`/${route.params.organization_slug}/registry/access-tokens/new`">New Access Token</UButton>
      </header>
      <div class="p-5">
        <RegistryAccessTokensPanel v-if="organizationId" :organization-id="organizationId" />
      </div>
    </div>
  </div>
</template>
