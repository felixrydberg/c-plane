<script setup lang="ts">
import { h } from 'vue'
import type { Repository } from '@cplane/sdk'
import type { TableColumn } from '@nuxt/ui'
import { FetchError } from 'ofetch'
import { ICONS } from '~/utils/icons'

defineOptions({ name: 'OrganizationRegistryPage' })

const store = useStore()
const route = useRoute()
const config = useRuntimeConfig()
const organizationId = computed(() => store.organization?.id ?? '')
const organizationSlug = computed(() => store.organization?.slug ?? '')
const registryHost = computed(() => config.public.registryHost)
const repositoriesUrl = computed(() => organizationId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/registry/repositories` as const
  : '')
const toast = useToast()
const { data: repositories, status, refresh: refreshRepositories } = await useCplaneFetch(repositoriesUrl, { default: () => [] })
const UButton = resolveComponent('UButton')
const UDropdownMenu = resolveComponent('UDropdownMenu')
const selectedRepository = ref<Repository | null>(null)
const deleteModalOpen = ref(false)
const deleting = ref(false)
const refreshing = ref(false)
const search = ref('')

const filteredRepositories = computed(() => {
  const query = search.value.trim().toLowerCase()
  return query ? repositories.value.filter(repository => repository.name.toLowerCase().includes(query)) : repositories.value
})

async function reloadRepositories() {
  refreshing.value = true
  try {
    await refreshRepositories()
  } finally {
    refreshing.value = false
  }
}

function confirmDelete(repository: Repository) {
  selectedRepository.value = repository
  deleteModalOpen.value = true
}

async function deleteRepository() {
  if (!selectedRepository.value || !organizationId.value) return
  deleting.value = true
  try {
    await cplaneFetch(`/api/organization/${organizationId.value as ':organization_id'}/registry/repositories/${selectedRepository.value.id as ':repository_id'}` as const, { method: 'DELETE' })
    toast.add({ title: 'Repository and images deleted', color: 'success' })
    deleteModalOpen.value = false
    selectedRepository.value = null
    await refreshRepositories()
  } catch (error) {
    const message = error instanceof FetchError ? error.data?.message : undefined
    toast.add({ title: message || 'Failed to delete repository and images', color: 'error' })
  } finally {
    deleting.value = false
  }
}

const columns: TableColumn<Repository>[] = [
  {
    accessorKey: 'name',
    header: 'Repository',
    cell: ({ row }) => h('span', { class: 'break-all font-mono text-sm' }, row.original.name),
  },
  {
    id: 'reference',
    header: 'Reference',
    cell: ({ row }) => h('span', { class: 'break-all font-mono text-xs text-muted' }, `${registryHost.value}/${organizationSlug.value}/${row.original.name}`),
  },
  {
    accessorKey: 'created_at',
    header: 'Created',
    cell: ({ row }) => new Date(row.original.created_at).toLocaleDateString(),
  },
  {
    id: 'actions',
    header: '',
    meta: { class: { th: 'text-right', td: 'text-right' } },
    cell: ({ row }) => h('div', { class: 'flex justify-end' }, h(UDropdownMenu, {
      items: [[{
        label: 'Delete',
        icon: ICONS.trash,
        color: 'error' as const,
        onSelect: () => confirmDelete(row.original),
      }]],
      size: 'sm',
      content: { align: 'end' },
    }, {
      default: () => h(UButton, {
        icon: ICONS.more,
        color: 'neutral',
        variant: 'ghost',
        size: 'xs',
        'aria-label': 'Repository actions',
        onClick: (event: MouseEvent) => event.stopPropagation(),
      }),
    })),
  },
]
</script>

<template>
  <div class="flex w-full max-w-375 flex-col gap-4 mx-auto">
    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <UiPageEyebrow label="Storage &amp; Databases" />
        <h1 class="text-2xl font-semibold">S2 - Registry</h1>
        <p class="text-muted text-sm mt-1">Private repositories for {{ organizationSlug }}.</p>
      </div>
      <div class="flex flex-wrap justify-end gap-2">
        <UButton :icon="ICONS.authentication" color="neutral" variant="ghost" :to="`/${organizationSlug}/registry/access-tokens`">Manage access tokens</UButton>
        <UButton :icon="ICONS.plus" color="primary" :to="`/${route.params.organization_slug}/registry/new`">New repository</UButton>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <UInput
        v-model="search"
        icon="i-heroicons:magnifying-glass"
        placeholder="Search repositories..."
        aria-label="Search repositories"
        class="min-w-0 flex-1"
      />
      <UButton
        :icon="ICONS.refresh"
        variant="ghost"
        color="neutral"
        :loading="refreshing"
        aria-label="Reload repositories"
        @click="reloadRepositories"
      />
    </div>

    <UiTable :status="status" :items="filteredRepositories" :columns="columns" disable-header>
      <template #empty>
        <div class="flex flex-col items-center justify-center gap-3 py-14 text-center">
          <UIcon :name="ICONS.registry" class="size-10 text-muted" />
          <p class="text-muted">{{ search ? 'No matching repositories.' : 'No repositories yet.' }}</p>
          <p v-if="!search" class="text-dimmed text-sm">Create your first repository before pushing an image.</p>
        </div>
      </template>
    </UiTable>

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
