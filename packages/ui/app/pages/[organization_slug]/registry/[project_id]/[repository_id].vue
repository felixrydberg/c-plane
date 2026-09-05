<script setup lang="ts">
import { h } from 'vue'
import type { Repository } from '@cplane/sdk'
import type { TableColumn } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'
import { getErrorMessage } from '~/utils/errors'

const store = useStore()
const route = useRoute()
const toast = useToast()
const config = useRuntimeConfig()
const organizationId = computed(() => store.organization?.id || '')
const organizationSlug = computed(() => store.organization?.slug ?? '')
const projectId = computed(() => route.params.project_id?.toString() ?? '')
const repositoryId = computed(() => route.params.repository_id?.toString() ?? '')
const project = computed(() => store.projects.find(item => item.id === projectId.value))
const canManage = computed(() => ['owner', 'admin'].includes(store.organization?.member?.role ?? ''))
const registryUrl = computed(() => organizationId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/registry` as const
  : '')
const repositoriesUrl = computed(() => organizationId.value && projectId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/registry/repositories` as const
  : '')
const tagsUrl = computed(() => organizationId.value && projectId.value && repositoryId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/registry/repositories/${repositoryId.value as ':repository_id'}/tags` as const
  : '')
const { data: managedRegistry } = await useCplaneFetch(registryUrl, { default: () => null })
const { data: repositories } = await useCplaneFetch(repositoriesUrl, { default: () => [] })
const { data: tagsResponse, status, error, refresh: refreshTags } = await useCplaneFetch<{ tags: string[] }>(tagsUrl, {
  default: () => ({ tags: [] }),
})

const repository = computed(() => repositories.value.find(item => item.id === repositoryId.value) as Repository | undefined)

watch(repository, () => {
  if (!repository.value?.name || !projectId.value || !organizationSlug.value) return
  store.setBreadcrumbs([
    { label: 'Registry', to: `/${organizationSlug.value}/registry/${projectId.value}` },
    { label: repository.value.name },
  ])
}, { immediate: true })

const registryIsActive = computed(() => managedRegistry.value?.status === 'active')
const projectRegistryName = computed(() => project.value?.name
  ?.trim()
  .replace(/[^a-zA-Z0-9]+/g, '-')
  .replace(/^-+|-+$/g, '')
  .toLowerCase() || 'project')
const baseReference = computed(() => repository.value
  ? `${config.public.registryHost}/${organizationSlug.value}/${projectRegistryName.value}/${repository.value.name}`
  : '')
const pullCommand = computed(() => `docker pull ${baseReference.value || '<repository>'}:<tag>`)

const search = ref('')
const refreshing = ref(false)
const offset = ref(0)
const tableReset = ref(0)
const TAG_PAGE_SIZE = 25
const selected = ref<string | null>(null)
const deleteModalOpen = ref(false)
const deleting = ref(false)
const copying = ref(false)

const filteredTags = computed(() => {
  const query = search.value.trim().toLowerCase()
  const tags = tagsResponse.value.tags
  return query ? tags.filter(tag => tag.toLowerCase().includes(query)) : tags
})

watch(search, () => { offset.value = 0; tableReset.value += 1 })

const tableKey = computed(() => `${search.value}:${tableReset.value}`)
const paginatedTags = computed(() => filteredTags.value.slice(offset.value, offset.value + TAG_PAGE_SIZE))
const showPagination = computed(() => filteredTags.value.length > TAG_PAGE_SIZE)

const tagStats = computed(() => [
  { label: 'Repository', value: repository.value?.name ?? '—' },
  { label: 'Project', value: project.value?.name ?? '—' },
  { label: 'Tags', value: String(tagsResponse.value.tags.length) },
])

const loadError = computed(() => getErrorMessage(error.value, 'The container registry is unavailable'))

function pullReference(tag: string) {
  return `${baseReference.value}:${tag}`
}

async function copyText(value: string, title: string) {
  copying.value = true
  try {
    await navigator.clipboard.writeText(value)
    toast.add({ title, color: 'success' })
  } catch {
    toast.add({ title: 'Failed to copy to clipboard', color: 'error' })
  } finally {
    copying.value = false
  }
}

async function reloadTags() {
  refreshing.value = true
  try {
    await refreshTags()
    offset.value = 0
    tableReset.value += 1
    if (error.value) {
      toast.add({ title: 'Failed to reload tags', color: 'error' })
    }
  } finally {
    refreshing.value = false
  }
}

function confirmDelete(tag: string) {
  selected.value = tag
  deleteModalOpen.value = true
}

async function deleteSelected() {
  if (!selected.value || !organizationId.value) return
  deleting.value = true
  try {
    await cplaneFetch(`/api/organization/${organizationId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/registry/repositories/${repositoryId.value as ':repository_id'}/tags/${selected.value as ':tag'}` as const, { method: 'DELETE' })
    toast.add({ title: 'Tag deleted', color: 'success' })
    deleteModalOpen.value = false
    selected.value = null
    await refreshTags()
  } catch {
    toast.add({ title: 'Failed to delete tag', color: 'error' })
  } finally {
    deleting.value = false
  }
}

const UButton = resolveComponent('UButton')
const UDropdownMenu = resolveComponent('UDropdownMenu')

const columns: TableColumn<string>[] = [
  {
    accessorKey: 'name',
    header: 'Tag',
    cell: ({ row }) => h('span', { class: 'break-all font-mono text-sm' }, row.original),
  },
  {
    id: 'actions',
    header: '',
    meta: { class: { th: 'text-right', td: 'text-right' } },
    cell: ({ row }) => {
      const tag = row.original
      const items = [[{
        label: 'Copy pull reference',
        icon: ICONS.link,
        onSelect: () => copyText(pullReference(tag), 'Pull reference copied'),
      }]]
      if (canManage.value) {
        items.push([{
          label: 'Delete',
          icon: ICONS.trash,
          color: 'error' as const,
          disabled: !registryIsActive.value,
          onSelect: () => confirmDelete(tag),
        }])
      }
      return h('div', { class: 'flex justify-end' }, h(UDropdownMenu, {
        items,
        size: 'sm',
        content: { align: 'end' },
      }, {
        default: () => h(UButton, {
          icon: ICONS.more,
          color: 'neutral',
          variant: 'ghost',
          size: 'xs',
          'aria-label': 'Tag actions',
        }),
      }))
    },
  },
]
</script>

<template>
  <div class="flex w-full max-w-375 flex-col gap-5 mx-auto">
    <div class="border-b border-default/60 pb-5">
      <UiPageEyebrow label="Storage &amp; Databases" />
      <h1 class="mt-2 text-2xl font-semibold">{{ repository?.name ?? 'Repository' }}</h1>
      <p class="mt-1 text-sm text-muted">Images in {{ project?.name ?? projectId }}.</p>
    </div>

    <UAlert
      v-if="!managedRegistry"
      color="warning"
      variant="subtle"
      icon="i-heroicons:exclamation-triangle"
      title="Managed Registry is not activated"
      description="Activate it in organization Registry settings before pushing images."
      :actions="[{ label: 'Open Registry settings', to: `/${organizationSlug}/settings/registry` }]"
    />
    <UAlert
      v-else-if="!registryIsActive"
      color="warning"
      variant="subtle"
      icon="i-heroicons:exclamation-triangle"
      title="Registry maintenance in progress"
      description="Pushes and tag changes are temporarily unavailable. Pulls and tag browsing remain available."
      :actions="[{ label: 'View maintenance', to: `/${organizationSlug}/settings/registry` }]"
    />

    <div class="grid gap-x-8 gap-y-4 border-y border-default/60 py-4 sm:grid-cols-2 lg:grid-cols-3">
      <div v-for="stat in tagStats" :key="stat.label">
        <p class="text-xs text-muted">{{ stat.label }}</p>
        <p class="mt-1 truncate text-base font-semibold" :title="stat.value">{{ stat.value }}</p>
      </div>
    </div>

    <div class="flex flex-col gap-2 rounded-lg border border-default/60 bg-elevated/20 px-4 py-3 sm:flex-row sm:items-center">
      <code class="min-w-0 flex-1 truncate font-mono text-xs text-muted" :title="pullCommand">{{ pullCommand }}</code>
      <UButton :icon="ICONS.link" color="neutral" variant="ghost" size="xs" :loading="copying" @click="copyText(pullCommand, 'Pull command copied')">Copy</UButton>
    </div>

    <div class="flex items-center gap-2">
      <UInput v-model="search" icon="i-heroicons:magnifying-glass" placeholder="Search tags..." aria-label="Search tags" class="min-w-0 flex-1" />
      <UButton :icon="ICONS.refresh" color="neutral" :loading="refreshing" aria-label="Refresh tags" @click="reloadTags">Refresh</UButton>
    </div>

    <UiTable :key="tableKey" v-model:offset="offset" :status="status" :items="paginatedTags" :columns="columns" disable-header :pagination="showPagination" :total="filteredTags.length" :limit="TAG_PAGE_SIZE">
      <template #empty>
        <div v-if="error" class="flex flex-col items-center justify-center gap-3 py-14 text-center">
          <UIcon :name="ICONS.registry" class="size-10 text-muted" />
          <p class="text-muted">Unable to load tags for this repository.</p>
          <p class="text-sm text-dimmed">{{ loadError }}</p>
          <UButton :icon="ICONS.refresh" variant="ghost" color="neutral" :loading="refreshing" aria-label="Retry loading tags" @click="reloadTags" />
        </div>
        <div v-else class="flex flex-col items-center justify-center gap-3 py-14 text-center">
          <UIcon :name="ICONS.registry" class="size-10 text-muted" />
          <p class="text-muted">{{ search ? 'No matching tags.' : 'No tags yet.' }}</p>
          <p v-if="!search && baseReference" class="break-all font-mono text-xs text-dimmed">docker push {{ baseReference }}:&lt;tag&gt;</p>
        </div>
      </template>
    </UiTable>

    <UModal v-model:open="deleteModalOpen" title="Delete tag" description="The manifest is untagged from this repository.">
      <template #body>
        <div class="space-y-4">
          <p class="text-sm">Are you sure you want to delete <strong>{{ selected }}</strong>?</p>
          <p class="text-sm text-muted">Storage space is reclaimed by the next registry garbage collection.</p>
          <div class="flex justify-end gap-3 pt-2">
            <UButton color="neutral" variant="ghost" :disabled="deleting" @click="deleteModalOpen = false">Cancel</UButton>
            <UButton :icon="ICONS.trash" color="error" :loading="deleting" @click="deleteSelected">Delete</UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>
