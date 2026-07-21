<script setup lang="ts">
import type { Repository } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'

defineOptions({ name: 'OrganizationRegistryPage' })

const store = useStore()
const route = useRoute()
const config = useRuntimeConfig()
const organizationId = computed(() => store.organization?.id ?? '')
const organizationSlug = computed(() => store.organization?.slug ?? '')
const registryHost = computed(() => config.public.registryHost)
const repositoriesUrl = computed(() => organizationId.value
  ? `/api/cplane/organization/${organizationId.value as ':organization_id'}/registry/repositories` as const
  : '')
const toast = useToast()
const { data: repositories, refresh: refreshRepositories } = await useFetch(repositoriesUrl, { default: () => [] })
const selectedRepository = ref<Repository | null>(null)
const deleteModalOpen = ref(false)
const deleting = ref(false)

function confirmDelete(repository: Repository) {
  selectedRepository.value = repository
  deleteModalOpen.value = true
}

async function deleteRepository() {
  if (!selectedRepository.value || !organizationId.value) return
  deleting.value = true
  try {
    await $fetch(`/api/cplane/organization/${organizationId.value as ':organization_id'}/registry/repositories/${selectedRepository.value.id as ':repository_id'}` as const, { method: 'DELETE' })
    toast.add({ title: 'Repository and images deleted', color: 'success' })
    deleteModalOpen.value = false
    selectedRepository.value = null
    await refreshRepositories()
  } catch {
    toast.add({ title: 'Failed to delete repository and images', color: 'error' })
  } finally {
    deleting.value = false
  }
}
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
      <div class="flex items-center justify-between gap-3 border-b border-default p-4">
        <div>
          <h2 class="font-semibold">{{ repository.name }}</h2>
          <p class="mt-1 break-all font-mono text-xs text-muted">{{ registryHost }}/{{ organizationSlug }}/{{ repository.name }}</p>
        </div>
        <UButton :icon="ICONS.trash" color="error" size="sm" @click="confirmDelete(repository)">Delete</UButton>
      </div>
    </section>

    <UModal v-model:open="deleteModalOpen" title="Delete repository" description="This permanently deletes the repository images and access permissions.">
      <template #body>
        <div class="space-y-4">
          <p class="text-sm">Are you sure you want to delete <strong>{{ selectedRepository?.name }}</strong>?</p>
          <div class="flex justify-end gap-3 pt-2">
            <UButton color="neutral" variant="ghost" :disabled="deleting" @click="deleteModalOpen = false">Cancel</UButton>
            <UButton :icon="ICONS.trash" color="error" :loading="deleting" @click="deleteRepository">Delete</UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>
