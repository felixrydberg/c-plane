<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const organizationId = computed(() => store.organization?.id ?? '')
const isOwner = computed(() => store.organization?.member?.role === 'owner')
</script>

<template>
  <div class="flex w-full max-w-375 flex-col gap-5 mx-auto">
    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <UiBackLink label="S2 - Registry" :to="`/${route.params.organization_slug}/registry`" />
        <UiPageEyebrow label="Storage &amp; Databases" />
        <h1 class="mt-2 text-2xl font-semibold">Access Tokens</h1>
        <p class="mt-1 text-sm text-muted">Credentials for container clients and CI pipelines in this organization.</p>
      </div>
      <UButton v-if="isOwner" :icon="ICONS.plus" color="primary" :to="`/${route.params.organization_slug}/registry/access-tokens/new`">New Access Token</UButton>
    </div>
    <RegistryAccessTokensPanel v-if="organizationId" :organization-id="organizationId" />
  </div>
</template>
