<script setup lang="ts">
import { h } from 'vue'
import type { Bucket } from '@cplane/sdk'
import type { TableColumn, TableRow } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const toast = useToast()
const organizationId = computed(() => store.organization?.id || '')
const projectId = computed(() => route.params.project_id as string)
const bucketsUrl = computed(() => organizationId.value
  ? `/api/cplane/organization/${organizationId.value as ':organization_id'}/storage/buckets` as const
  : '')
const selectedBucket = ref<Bucket | null>(null)
const deleteModalOpen = ref(false)
const deleting = ref(false)
const refreshing = ref(false)
const search = ref('')
const { data: buckets, status, refresh: refreshBuckets } = await useFetch(bucketsUrl, {
  default: () => [],
  query: { project_id: projectId },
})

const UButton = resolveComponent('UButton')
const UDropdownMenu = resolveComponent('UDropdownMenu')
const NuxtLink = resolveComponent('NuxtLink')

function bucketUrl(bucketId: string) {
  return `/${route.params.organization_slug}/storage/${projectId.value}/${bucketId}`
}

function openBucket(row: TableRow<Bucket>) {
  return navigateTo(bucketUrl(row.original.id))
}

const filteredBuckets = computed(() => {
  const query = search.value.trim().toLowerCase()
  return query ? buckets.value.filter(bucket => bucket.name.toLowerCase().includes(query)) : buckets.value
})

async function reloadBuckets() {
  refreshing.value = true
  try {
    await refreshBuckets()
  } finally {
    refreshing.value = false
  }
}

function confirmDelete(bucket: Bucket) {
  selectedBucket.value = bucket
  deleteModalOpen.value = true
}

async function deleteBucket() {
  if (!selectedBucket.value || !organizationId.value) return
  deleting.value = true
  try {
    await $fetch(`/api/cplane/organization/${organizationId.value as ':organization_id'}/storage/buckets/${selectedBucket.value.id as ':bucket_id'}` as const, { method: 'DELETE' })
    toast.add({ title: 'Bucket deleted', color: 'success' })
    deleteModalOpen.value = false
    selectedBucket.value = null
    await refreshBuckets()
  } catch {
    toast.add({ title: 'Failed to delete bucket', description: 'The bucket must be empty before it can be deleted.', color: 'error' })
  } finally {
    deleting.value = false
  }
}

const columns: TableColumn<Bucket>[] = [
  {
    accessorKey: 'name',
    header: 'Bucket',
    cell: ({ row }) => h(NuxtLink, {
      to: bucketUrl(row.original.id),
      class: 'truncate font-medium text-primary group-hover:underline group-hover:underline-offset-4',
    }, () => row.original.name),
  },
  {
    id: 'objects',
    header: 'Objects',
    cell: () => '0',
  },
  {
    id: 'size',
    header: 'Size',
    cell: () => '0 B',
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
        'aria-label': 'Bucket actions',
        onClick: (event: MouseEvent) => event.stopPropagation(),
      }),
    })),
  },
]
</script>

<template>
  <div class="flex w-full max-w-375 flex-col gap-4 mx-auto">
    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div><UiPageEyebrow label="Storage &amp; Databases" /><h1 class="text-2xl font-semibold">S1 - Object Storage</h1><p class="text-muted text-sm mt-1">High-performance object storage for files and objects.</p></div>
      <div class="flex flex-wrap justify-end gap-2">
        <UButton :icon="ICONS.authentication" variant="ghost" color="neutral" :to="`/${route.params.organization_slug}/storage/${projectId}/access-tokens`">Manage access tokens</UButton>
        <UButton :icon="ICONS.plus" color="primary" :to="`/${route.params.organization_slug}/storage/${projectId}/new`">New bucket</UButton>
      </div>
    </div>
    <div class="flex items-center gap-2">
      <UInput
        v-model="search"
        icon="i-heroicons:magnifying-glass"
        placeholder="Search buckets..."
        aria-label="Search buckets"
        class="min-w-0 flex-1"
      />
      <UButton
        :icon="ICONS.refresh"
        variant="ghost"
        color="neutral"
        :loading="refreshing"
        aria-label="Reload buckets"
        @click="reloadBuckets"
      />
    </div>
    <UiTable :status="status" :items="filteredBuckets" :columns="columns" disable-header selectable @select="openBucket">
      <template #empty>
        <div class="flex flex-col items-center justify-center gap-3 py-14 text-center">
          <UIcon :name="ICONS.storage" class="size-10 text-muted" />
          <p class="text-muted">No buckets yet.</p>
          <p class="text-dimmed text-sm">Create your first bucket to start storing objects.</p>
        </div>
      </template>
    </UiTable>
    <UModal v-model:open="deleteModalOpen" title="Delete bucket" description="This deletes the physical provider bucket and its logical record. The bucket must be empty.">
      <template #body>
        <div class="space-y-4">
          <p class="text-sm">Are you sure you want to delete <strong>{{ selectedBucket?.name }}</strong>?</p>
          <div class="flex justify-end gap-3 pt-2">
            <UButton color="neutral" variant="ghost" :disabled="deleting" @click="deleteModalOpen = false">Cancel</UButton>
            <UButton :icon="ICONS.trash" color="error" :loading="deleting" @click="deleteBucket">Delete</UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>
