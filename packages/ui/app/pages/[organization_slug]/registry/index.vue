<script setup lang="ts">
import { ICONS } from '~/utils/icons'

defineOptions({ name: 'OrganizationRegistryPage' })

interface Repository { id: string; name: string; created_at: string }

const store = useStore()
const route = useRoute()
const config = useRuntimeConfig()
const organizationId = computed(() => store.organization?.id ?? '')
const organizationSlug = computed(() => store.organization?.slug ?? '')
const registryHost = computed(() => config.public.registryHost)
const repositoriesUrl = computed(() => organizationId.value
  ? `/api/backend/organization/${organizationId.value}/registry/repositories`
  : '')
const { data: repositories } = await useFetch<Repository[]>(repositoriesUrl, { default: () => [] })
</script>

<template>
  <div class="flex w-full max-w-[1500px] flex-col gap-5 mx-auto">
    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <h1 class="text-2xl font-semibold">Registry</h1>
        <p class="text-muted text-sm mt-1">Private repositories for {{ organizationSlug }}.</p>
      </div>
      <div class="flex flex-wrap justify-end gap-2">
        <UButton :icon="ICONS.authentication" color="neutral" variant="solid" :to="`/${organizationSlug}/registry/access-tokens`">Manage access tokens</UButton>
        <UButton :icon="ICONS.plus" color="primary" :to="`/${route.params.organization_slug}/registry/new`">New repository</UButton>
      </div>
    </div>

    <div v-if="!repositories.length" class="flex flex-col items-center justify-center py-14 gap-3 text-center rounded-lg border border-dashed border-default bg-transparent">
      <UIcon :name="ICONS.registry" class="size-10 text-muted" />
      <p class="text-muted">No repositories yet.</p>
      <p class="text-dimmed text-sm">Create your first repository before pushing an image.</p>
    </div>

    <section v-for="repository in repositories" :key="repository.id" class="overflow-hidden rounded-lg border border-dashed border-default bg-transparent">
      <div class="p-4">
        <h2 class="font-semibold">{{ repository.name }}</h2>
        <p class="mt-1 break-all font-mono text-xs text-muted">{{ registryHost }}/{{ organizationSlug }}/{{ repository.name }}</p>
      </div>
    </section>
  </div>
</template>
