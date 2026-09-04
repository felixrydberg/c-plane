<script setup lang="ts">
import { h } from 'vue'
import type { Repository } from '@cplane/sdk'
import type { TableColumn } from '@nuxt/ui'
import { FetchError } from 'ofetch'
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const config = useRuntimeConfig()
const toast = useToast()
const organizationId = computed(() => store.organization?.id ?? '')
const organizationSlug = computed(() => store.organization?.slug ?? '')
const projectId = computed(() => route.params.project_id?.toString() ?? '')
const project = computed(() => store.projects.find(item => item.id === projectId.value))
const canManage = computed(() => ['owner', 'admin'].includes(store.organization?.member?.role ?? ''))
const registryUrl = computed(() => organizationId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/registry` as const
  : '')
const repositoriesUrl = computed(() => organizationId.value && projectId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/registry/repositories` as const
  : '')
const { data: managedRegistry } = await useCplaneFetch(registryUrl, { default: () => null })
const { data: repositories, status, refresh: refreshRepositories } = await useCplaneFetch(repositoriesUrl, { default: () => [] })
const selected = ref<Repository | null>(null)
const deleting = ref(false)
const refreshing = ref(false)
const search = ref('')
const registryIsActive = computed(() => managedRegistry.value?.status === 'active')
const filteredRepositories = computed(() => {
  const query = search.value.trim().toLowerCase()
  return query ? repositories.value.filter(repository => repository.name.toLowerCase().includes(query)) : repositories.value
})
const projectRegistryName = computed(() => project.value?.name
  ?.trim()
  .replace(/[^a-zA-Z0-9]+/g, '-')
  .replace(/^-+|-+$/g, '')
  .toLowerCase() || 'project')
const reference = (repository: Repository) => `${config.public.registryHost}/${organizationSlug.value}/${projectRegistryName.value}/${repository.name}`
const UButton = resolveComponent('UButton')

const columns: TableColumn<Repository>[] = [
  {
    accessorKey: 'name',
    header: 'Repository',
    cell: ({ row }) => h('span', { class: 'break-all font-mono text-sm' }, row.original.name),
  },
  {
    id: 'reference',
    header: 'Reference',
    meta: { class: { th: 'hidden lg:table-cell', td: 'hidden lg:table-cell' } },
    cell: ({ row }) => h('span', { class: 'break-all font-mono text-xs text-muted' }, reference(row.original)),
  },
  {
    accessorKey: 'created_at',
    header: 'Created',
    meta: { class: { th: 'hidden sm:table-cell', td: 'hidden sm:table-cell' } },
    cell: ({ row }) => new Date(row.original.created_at).toLocaleDateString('sv-SE', { timeZone: 'UTC' }),
  },
  {
    id: 'actions',
    header: '',
    meta: { class: { th: 'text-right', td: 'text-right' } },
    cell: ({ row }) => canManage.value ? h(UButton, {
      icon: ICONS.trash,
      color: 'error',
      size: 'sm',
      disabled: !registryIsActive.value,
      onClick: () => { selected.value = row.original },
    }, { default: () => 'Delete' }) : null,
  },
]

async function deleteRepository() {
  if (!selected.value) return
  deleting.value = true
  try {
    await cplaneFetch(`/api/organization/${organizationId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/registry/repositories/${selected.value.id as ':repository_id'}` as const, { method: 'DELETE' })
    toast.add({ title: 'Repository deleted', color: 'success' })
    selected.value = null
    await refresh()
  } catch (error) {
    const message = error instanceof FetchError ? error.data?.message : undefined
    toast.add({ title: message || 'Failed to delete repository', color: 'error' })
  } finally {
    deleting.value = false
  }
}

async function refresh() {
  if (refreshing.value) return
  refreshing.value = true
  try {
    await refreshRepositories()
  } finally {
    refreshing.value = false
  }
}
</script>

<template>
  <div class="flex w-full max-w-375 flex-col gap-5 mx-auto">
    <header class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <UiPageEyebrow label="Storage &amp; Databases" />
        <h1 class="mt-2 text-2xl font-semibold">Registry</h1>
        <p class="mt-1 text-sm text-muted">Private repositories for {{ project?.name ?? projectId }}.</p>
      </div>
      <div class="flex flex-wrap gap-2">
        <UButton :icon="ICONS.authentication" color="neutral" :to="`/${organizationSlug}/registry/${projectId}/access-tokens`">Manage access tokens</UButton>
        <UButton :icon="ICONS.plus" color="primary" :to="`/${organizationSlug}/registry/${projectId}/new`" :disabled="!registryIsActive">New repository</UButton>
      </div>
    </header>

    <UAlert
      v-if="!managedRegistry"
      color="warning"
      variant="subtle"
      icon="i-heroicons:exclamation-triangle"
      title="Managed Registry is not activated"
      description="Activate it in organization Registry settings before creating repositories."
      :actions="[{ label: 'Open Registry settings', to: `/${organizationSlug}/settings/registry` }]"
    />
    <UAlert
      v-else-if="!registryIsActive"
      color="warning"
      variant="subtle"
      icon="i-heroicons:exclamation-triangle"
      title="Registry maintenance in progress"
      description="Pulls, pushes, and repository changes are temporarily unavailable."
      :actions="[{ label: 'View maintenance', to: `/${organizationSlug}/settings/registry` }]"
    />

    <div class="flex items-center gap-2">
      <UInput v-model="search" icon="i-heroicons:magnifying-glass" placeholder="Search repositories..." class="min-w-0 flex-1" />
      <UButton :icon="ICONS.refresh" color="neutral" :loading="refreshing" @click="refresh">Refresh</UButton>
    </div>

    <UiTable :status="status" :items="filteredRepositories" :columns="columns" disable-header>
      <template #empty>
        <div class="flex flex-col items-center justify-center gap-3 py-14 text-center">
          <UIcon :name="ICONS.registry" class="size-10 text-muted" />
          <p class="text-muted">{{ search ? 'No matching repositories.' : 'No repositories yet.' }}</p>
          <p v-if="!search" class="text-sm text-dimmed">Create your first repository before pushing an image.</p>
        </div>
      </template>
    </UiTable>

    <UModal :open="Boolean(selected)" title="Delete repository" description="The repository will be removed from this project." @update:open="!$event && (selected = null)">
      <template #body>
        <div class="space-y-4">
          <p class="text-sm">Delete <strong>{{ selected?.name }}</strong> from <strong>{{ project?.name ?? projectId }}</strong>?</p>
          <p v-if="selected" class="break-all font-mono text-xs text-muted">{{ reference(selected) }}</p>
          <div class="flex justify-end gap-3">
            <UButton color="neutral" variant="ghost" @click="selected = null">Cancel</UButton>
            <UButton :icon="ICONS.trash" color="error" :loading="deleting" @click="deleteRepository">Delete repository</UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>
