<script setup lang="ts">
import { h } from 'vue'
import type { Bucket } from '@cplane/sdk'
import type { TableColumn } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

type BucketObject = {
  key: string
  size: number
  last_modified: string | null
  etag: string | null
}

type BucketObjectsPage = {
  folders: string[]
  objects: BucketObject[]
  next_continuation_token: string | null
}

type DeleteTarget = {
  type: 'object' | 'folder'
  key: string
}

type BucketRow = {
  type: 'object' | 'folder'
  key: string
  name: string
  last_modified: string | null
  size: number | null
  object?: BucketObject
}

const store = useStore()
const route = useRoute()
const toast = useToast()
const organizationId = computed(() => store.organization?.id || '')
const projectId = computed(() => route.params.project_id as string)
const bucketId = computed(() => route.params.bucket_id as string)
const prefix = computed(() => typeof route.query.prefix === 'string' ? route.query.prefix : '')
const bucketsUrl = computed(() => organizationId.value
  ? `/api/cplane/organization/${organizationId.value as ':organization_id'}/storage/buckets` as const
  : '')
const objectsUrl = computed(() => organizationId.value
  ? `/api/cplane/organization/${organizationId.value as ':organization_id'}/storage/buckets/${bucketId.value as ':bucket_id'}/objects` as const
  : '')
const downloadUrl = computed(() => organizationId.value
  ? `/api/cplane/organization/${organizationId.value as ':organization_id'}/storage/buckets/${bucketId.value as ':bucket_id'}/objects/download` as const
  : '')
const { data: buckets } = await useFetch(bucketsUrl, {
  default: () => [],
  query: { project_id: projectId },
})
const { data: initialPage, status, error, refresh } = await useFetch<BucketObjectsPage>(objectsUrl, {
  default: () => ({ folders: [], objects: [], next_continuation_token: null }),
  query: () => ({ prefix: prefix.value || undefined }),
})

const bucket = computed(() => buckets.value.find(item => item.id === bucketId.value) as Bucket | undefined)
const folders = ref<string[]>([])
const objects = ref<BucketObject[]>([])
const nextContinuationToken = ref<string | null>(null)
const pageNumber = ref(1)
const pageCache = ref<BucketObjectsPage[]>([])
const loadingPage = ref(false)
const downloading = ref<string | null>(null)
const refreshing = ref(false)
const deleteTarget = ref<DeleteTarget | null>(null)
const deleteModalOpen = ref(false)
const deleting = ref(false)

watch(initialPage, (page) => {
  const firstPage = page ?? { folders: [], objects: [], next_continuation_token: null }
  pageCache.value = [firstPage]
  pageNumber.value = 1
  applyPage(firstPage)
}, { immediate: true })

const breadcrumbs = computed(() => {
  const parts = prefix.value.split('/').filter(Boolean)
  return parts.map((name, index) => ({
    name,
    prefix: `${parts.slice(0, index + 1).join('/')}/`,
  }))
})

function backUrl() {
  return `/${route.params.organization_slug}/storage/${projectId.value}`
}

function parentPrefix() {
  const parts = prefix.value.split('/').filter(Boolean)
  parts.pop()
  return parts.length ? `${parts.join('/')}/` : ''
}

async function openPrefix(nextPrefix: string) {
  await navigateTo({ query: nextPrefix ? { prefix: nextPrefix } : {} })
}

function applyPage(page: BucketObjectsPage) {
  folders.value = page.folders
  objects.value = page.objects
  nextContinuationToken.value = page.next_continuation_token
}

async function goToPage(targetPage: number) {
  if (targetPage < 1 || loadingPage.value) return
  const cachedPage = pageCache.value[targetPage - 1]
  if (cachedPage) {
    pageNumber.value = targetPage
    applyPage(cachedPage)
    return
  }
  if (targetPage !== pageNumber.value + 1 || !nextContinuationToken.value) return
  loadingPage.value = true
  try {
    const page = await $fetch<BucketObjectsPage>(objectsUrl.value, {
      query: { prefix: prefix.value || undefined, continuation_token: nextContinuationToken.value },
    })
    pageCache.value[targetPage - 1] = page
    pageNumber.value = targetPage
    applyPage(page)
  } catch {
    toast.add({ title: 'Failed to load next page', color: 'error' })
  } finally {
    loadingPage.value = false
  }
}

async function reloadObjects() {
  refreshing.value = true
  try {
    await refresh()
    if (error.value) {
      toast.add({ title: 'Failed to reload objects', color: 'error' })
    }
  } finally {
    refreshing.value = false
  }
}

const deleteTargetName = computed(() => {
  if (!deleteTarget.value) return ''
  if (deleteTarget.value.type === 'folder') {
    return deleteTarget.value.key.slice(prefix.value.length).replace(/\/$/, '')
  }
  return deleteTarget.value.key.slice(prefix.value.length)
})

function confirmDelete(type: DeleteTarget['type'], key: string) {
  deleteTarget.value = { type, key }
  deleteModalOpen.value = true
}

async function deleteSelected() {
  if (!deleteTarget.value) return
  const type = deleteTarget.value.type
  deleting.value = true
  try {
    const query = type === 'folder'
      ? { prefix: deleteTarget.value.key }
      : { key: deleteTarget.value.key }
    let continuationToken: string | undefined
    do {
      const response = await $fetch<{ next_continuation_token?: string | null }>(objectsUrl.value, {
        method: 'DELETE',
        query: { ...query, continuation_token: continuationToken },
      })
      continuationToken = response?.next_continuation_token ?? undefined
    } while (continuationToken)
    toast.add({
      title: type === 'folder' ? 'Folder deleted' : 'Object deleted',
      color: 'success',
    })
    deleteModalOpen.value = false
    deleteTarget.value = null
    await reloadObjects()
  } catch {
    toast.add({
      title: type === 'folder' ? 'Failed to delete folder' : 'Failed to delete object',
      color: 'error',
    })
  } finally {
    deleting.value = false
  }
}

async function download(object: BucketObject) {
  downloading.value = object.key
  try {
    const blob = await $fetch<Blob>(downloadUrl.value, { query: { key: object.key }, responseType: 'blob' })
    const url = URL.createObjectURL(blob)
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = object.key.split('/').filter(Boolean).pop() || 'download'
    document.body.append(anchor)
    anchor.click()
    anchor.remove()
    setTimeout(() => URL.revokeObjectURL(url), 0)
  } catch {
    toast.add({ title: 'Failed to download object', color: 'error' })
  } finally {
    downloading.value = null
  }
}

function formatSize(size: number) {
  if (size < 1024) return `${size} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let value = size / 1024
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unit]}`
}

const UButton = resolveComponent('UButton')
const UIcon = resolveComponent('UIcon')
const rows = computed<BucketRow[]>(() => [
  ...folders.value.map(key => ({
    type: 'folder' as const,
    key,
    name: key.slice(prefix.value.length).replace(/\/$/, ''),
    last_modified: null,
    size: null,
  })),
  ...objects.value.map(object => ({
    type: 'object' as const,
    key: object.key,
    name: object.key.slice(prefix.value.length),
    last_modified: object.last_modified,
    size: object.size,
    object,
  })),
])

const columns: TableColumn<BucketRow>[] = [
  {
    accessorKey: 'name',
    header: 'Name',
    cell: ({ row }) => {
      const item = row.original
      if (item.type === 'folder') {
        return h('button', {
          type: 'button',
          class: 'flex min-w-0 items-center gap-2 text-left hover:text-default',
          onClick: () => openPrefix(item.key),
        }, [
          h(UIcon, { name: ICONS.folder, class: 'size-4 shrink-0 text-muted' }),
          h('span', { class: 'truncate' }, item.name),
        ])
      }
      return h('span', { class: 'font-mono text-xs break-all' }, item.name)
    },
  },
  {
    accessorKey: 'last_modified',
    header: 'Modified',
    meta: { class: { th: 'hidden sm:table-cell', td: 'hidden sm:table-cell' } },
    cell: ({ row }) => row.original.last_modified
      ? new Date(row.original.last_modified).toLocaleString()
      : '—',
  },
  {
    accessorKey: 'size',
    header: 'Size',
    meta: { class: { th: 'hidden sm:table-cell text-right', td: 'hidden sm:table-cell text-right' } },
    cell: ({ row }) => row.original.size === null ? '—' : formatSize(row.original.size),
  },
  {
    id: 'actions',
    header: 'Actions',
    meta: { class: { th: 'text-right', td: 'text-right' } },
    cell: ({ row }) => {
      const item = row.original
      const buttons = item.type === 'folder'
        ? [h(UButton, {
            icon: ICONS.trash,
            color: 'error',
            size: 'xs',
            onClick: () => confirmDelete('folder', item.key),
          }, { default: () => 'Delete' })]
        : [
            h(UButton, {
              icon: ICONS.download,
              color: 'neutral',
              size: 'xs',
              loading: downloading.value === item.key,
              onClick: () => download(item.object!),
            }, { default: () => 'Download' }),
            h(UButton, {
              icon: ICONS.trash,
              color: 'error',
              size: 'xs',
              loading: deleting.value && deleteTarget.value?.key === item.key,
              onClick: () => confirmDelete('object', item.key),
            }, { default: () => 'Delete' }),
          ]
      return h('div', { class: 'flex justify-end gap-2' }, buttons)
    },
  },
]
</script>

<template>
  <div class="flex w-full max-w-[1500px] flex-col gap-5 mx-auto">
    <div class="border-b border-default/60 pb-5">
      <UiBackLink label="Buckets" :to="backUrl()" />
      <div class="mt-2 flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 class="text-2xl font-semibold">{{ bucket?.name || 'Bucket objects' }}</h1>
          <div class="mt-2 flex flex-wrap items-center gap-1 text-sm text-muted">
            <button class="hover:text-default" @click="openPrefix('')">{{ bucket?.name || 'Bucket' }}</button>
            <template v-for="item in breadcrumbs" :key="item.prefix">
              <span>/</span><button class="hover:text-default" @click="openPrefix(item.prefix)">{{ item.name }}</button>
            </template>
          </div>
        </div>
        <div class="flex flex-wrap gap-2">
          <UButton :icon="ICONS.refresh" color="neutral" :loading="refreshing" aria-label="Reload objects" @click="reloadObjects" />
        </div>
      </div>
    </div>

    <div v-if="status === 'pending'" class="flex items-center justify-center py-14 text-sm text-muted">Loading objects…</div>
    <div v-else-if="error" class="flex flex-col items-center justify-center gap-3 py-14 text-center">
      <p class="text-sm text-muted">Unable to load this bucket.</p>
      <UButton :icon="ICONS.refresh" color="neutral" :loading="refreshing" aria-label="Retry loading objects" @click="reloadObjects" />
    </div>
    <UiTable v-else :status="status" :items="rows" :columns="columns" disable-header>
      <template #empty>
        <div class="flex flex-col items-center justify-center gap-3 py-14 text-center">
          <UIcon :name="ICONS.folder" class="size-10 text-muted" />
          <p class="text-muted">This folder is empty.</p>
        </div>
      </template>
    </UiTable>
    <div v-if="pageNumber > 1 || nextContinuationToken" class="flex items-center justify-center gap-3">
      <UButton color="neutral" variant="ghost" :disabled="loadingPage || pageNumber === 1" @click="goToPage(pageNumber - 1)">Previous</UButton>
      <span class="text-sm text-muted">Page {{ pageNumber }}</span>
      <UButton color="neutral" variant="ghost" :loading="loadingPage" :disabled="!nextContinuationToken" @click="goToPage(pageNumber + 1)">Next</UButton>
    </div>

    <UModal v-model:open="deleteModalOpen" :title="deleteTarget?.type === 'folder' ? 'Delete folder' : 'Delete object'" :description="deleteTarget?.type === 'folder' ? 'This deletes every object under the folder prefix.' : 'This permanently deletes the object.'">
      <template #body>
        <div class="space-y-4">
          <p class="text-sm">Are you sure you want to delete <strong>{{ deleteTargetName }}</strong>?</p>
          <p v-if="deleteTarget?.type === 'folder'" class="text-sm text-muted">All objects inside this folder will be deleted.</p>
          <div class="flex justify-end gap-3 pt-2">
            <UButton color="neutral" variant="ghost" :disabled="deleting" @click="deleteModalOpen = false">Cancel</UButton>
            <UButton :icon="ICONS.trash" color="error" :loading="deleting" @click="deleteSelected">Delete</UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>
