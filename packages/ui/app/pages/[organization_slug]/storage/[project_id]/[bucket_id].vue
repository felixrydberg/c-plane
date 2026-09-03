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
const organizationSlug = computed(() => route.params.organization_slug?.toString() || '')
const organizationId = computed(() => store.organization?.id || '')
const projectId = computed(() => route.params.project_id as string)
const bucketId = computed(() => route.params.bucket_id as string)
const prefix = computed(() => typeof route.query.prefix === 'string' ? route.query.prefix : '')
const bucketsUrl = computed(() => organizationId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/storage/buckets` as const
  : '')
const objectsUrl = computed(() => organizationId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/storage/buckets/${bucketId.value as ':bucket_id'}/objects` as const
  : '')
const downloadUrl = computed(() => organizationId.value
  ? `/api/organization/${organizationId.value as ':organization_id'}/storage/buckets/${bucketId.value as ':bucket_id'}/objects/download` as const
  : '')
const { data: buckets } = await useCplaneFetch(bucketsUrl, {
  default: () => [],
  query: { project_id: projectId },
})
const { data: initialPage, status, error, refresh } = await useCplaneFetch<BucketObjectsPage>(objectsUrl, {
  default: () => ({ folders: [], objects: [], next_continuation_token: null }),
  query: () => ({ prefix: prefix.value || undefined }),
})

const bucket = computed(() => buckets.value.find(item => item.id === bucketId.value) as Bucket | undefined)

watch(bucket, () => {
  if (!bucket.value?.name || !projectId.value || !bucketId.value || !organizationSlug.value) return
  store.setBreadcrumbs([
    { label: 'Object Storage', to: `/${organizationSlug.value}/storage/${projectId.value}` },
    { label: bucket.value.name },
  ])
}, { immediate: true })

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
const activeView = ref<'objects' | 'usage'>('objects')

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
    const page = await cplaneFetch<BucketObjectsPage>(objectsUrl.value, {
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
      const response = await cplaneFetch<{ next_continuation_token?: string | null }>(objectsUrl.value, {
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
    const blob = await cplaneFetch<Blob>(downloadUrl.value, { query: { key: object.key }, responseType: 'blob' })
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
const UDropdownMenu = resolveComponent('UDropdownMenu')
const NuxtLink = resolveComponent('NuxtLink')
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

const viewTabs = [
  { label: 'Objects', value: 'objects', slot: 'objects' },
  { label: 'Usage', value: 'usage', slot: 'usage' },
]
const usageSummary = [
  { label: 'Average Storage', value: '0 B' },
  { label: 'Data Retrieved', value: '0 B' },
  { label: 'Class A Operations', value: '0' },
  { label: 'Class B Operations', value: '0' },
  { label: 'Request Distribution', value: '0' },
]
const usagePanels = [
  { label: 'Average Storage', values: [{ label: 'Total', value: '0 B', color: 'bg-neutral-400' }] },
  { label: 'Data Retrieved', values: [{ label: 'Total', value: '0 B', color: 'bg-neutral-400' }] },
  { label: 'Class A Operations', values: [{ label: 'Total', value: '0', color: 'bg-neutral-400' }] },
  { label: 'Class B Operations', values: [{ label: 'Total', value: '0', color: 'bg-neutral-400' }] },
]

const objectStats = computed(() => [
  { label: 'Region', value: bucket.value ? `${bucket.value.region.label} (${bucket.value.region.slug})` : '—' },
  { label: 'Objects', value: String(objects.value.length) },
  { label: 'Listed Size', value: formatSize(objects.value.reduce((total, object) => total + object.size, 0)) },
  { label: 'Class A Operations', value: '0' },
  { label: 'Class B Operations', value: '0' },
])

const columns: TableColumn<BucketRow>[] = [
  {
    accessorKey: 'name',
    header: 'Name',
    cell: ({ row }) => {
      const item = row.original
      if (item.type === 'folder') {
        return h(NuxtLink, {
          to: { query: { prefix: item.key } },
          class: 'flex min-w-0 items-center gap-2 text-left text-primary hover:underline hover:underline-offset-4',
        }, [
          h(UIcon, { name: ICONS.folder, class: 'size-4 shrink-0 text-muted' }),
          h('span', { class: 'truncate font-medium' }, item.name),
        ])
      }
      return h('span', { class: 'break-all font-mono text-xs text-default' }, item.name)
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
    meta: { class: { th: 'hidden sm:table-cell', td: 'hidden sm:table-cell' } },
    cell: ({ row }) => row.original.size === null ? '—' : formatSize(row.original.size),
  },
  {
    id: 'actions',
    header: '',
    meta: { class: { th: 'text-right', td: 'text-right' } },
    cell: ({ row }) => {
      const item = row.original
      const items = item.type === 'folder'
        ? [[{
            label: 'Delete',
            icon: ICONS.trash,
            color: 'error' as const,
            onSelect: () => confirmDelete('folder', item.key),
          }]]
        : [[{
            label: 'Download',
            icon: ICONS.download,
            loading: downloading.value === item.key,
            onSelect: () => download(item.object!),
          }], [{
            label: 'Delete',
            icon: ICONS.trash,
            color: 'error' as const,
            onSelect: () => confirmDelete('object', item.key),
          }]]
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
        }),
      }))
    },
  },
]
</script>

<template>
  <div class="flex w-full max-w-6xl flex-col gap-5 mx-auto">
    <div class="border-b border-default/60 pb-5">
      <UiPageEyebrow label="Storage &amp; Databases" />
      <h1 class="mt-2 text-2xl font-semibold">{{ bucket?.name ?? 'Bucket' }}</h1>
      <p class="mt-1 text-sm text-muted">Object Storage bucket.</p>
    </div>
    <UiTabs v-model="activeView" :items="viewTabs">
      <template #objects>
        <div class="space-y-4 pt-4">
          <div class="grid gap-x-8 gap-y-4 border-y border-default/60 py-4 sm:grid-cols-2 lg:grid-cols-5">
            <div v-for="stat in objectStats" :key="stat.label">
              <p class="text-xs text-muted">{{ stat.label }}</p>
              <p class="mt-1 text-base font-semibold">{{ stat.value }}</p>
            </div>
          </div>

          <nav v-if="bucket?.name" aria-label="Bucket path" class="flex min-w-0 items-center justify-between gap-3 text-sm text-muted">
            <div class="flex min-w-0 flex-wrap items-center gap-1">
              <button v-if="breadcrumbs.length" type="button" class="underline underline-offset-2 hover:text-default" @click="openPrefix('')">{{ bucket.name }}</button>
              <span v-else>{{ bucket.name }}</span>
              <span>/</span>
              <template v-for="item in breadcrumbs" :key="item.prefix">
                <span>{{ item.name }}</span>
                <span>/</span>
              </template>
            </div>
            <UButton :icon="ICONS.refresh" variant="ghost" color="neutral" :loading="refreshing" aria-label="Reload objects" @click="reloadObjects" />
          </nav>

          <div v-if="status === 'pending'" class="flex items-center justify-center py-14 text-sm text-muted">Loading objects…</div>
          <div v-else-if="error" class="flex flex-col items-center justify-center gap-3 py-14 text-center">
            <p class="text-sm text-muted">Unable to load this bucket.</p>
            <UButton :icon="ICONS.refresh" variant="ghost" color="neutral" :loading="refreshing" aria-label="Retry loading objects" @click="reloadObjects" />
          </div>
          <UiTable v-else :status="status" :items="rows" :columns="columns" disable-header>
            <template #empty>
              <div class="flex flex-col items-center justify-center gap-3 py-14 text-center">
                <UIcon :name="ICONS.folder" class="size-10 text-muted" />
                <p class="text-muted">This folder is empty.</p>
              </div>
            </template>
          </UiTable>
          <div v-if="pageNumber > 1 || nextContinuationToken" class="flex justify-center pt-1">
            <nav aria-label="Object pages" class="inline-flex items-center rounded-lg border border-default/60 bg-elevated/20 p-1 shadow-sm">
              <UButton
                color="neutral"
                variant="ghost"
                size="sm"
                :leading-icon="ICONS.chevronLeft"
                class="min-w-22 justify-center"
                :disabled="loadingPage || pageNumber === 1"
                @click="goToPage(pageNumber - 1)"
              >
                Previous
              </UButton>
              <span aria-current="page" class="mx-1 min-w-18 rounded-md border border-default/60 bg-default/10 px-3 py-1.5 text-center text-xs font-medium text-muted">
                Page {{ pageNumber }}
              </span>
              <UButton
                color="neutral"
                variant="ghost"
                size="sm"
                :trailing-icon="ICONS.chevronRight"
                class="min-w-18 justify-center"
                :loading="loadingPage"
                :disabled="!nextContinuationToken"
                @click="goToPage(pageNumber + 1)"
              >
                Next
              </UButton>
            </nav>
          </div>
        </div>
      </template>

      <template #usage>
        <div class="space-y-5 pt-4">
          <div class="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <h2 class="text-base font-semibold">Usage</h2>
              <p class="mt-1 text-sm text-muted">Storage and request activity for this bucket.</p>
            </div>
            <UButton :icon="ICONS.calendar" color="neutral" variant="outline">Last 24 hours</UButton>
          </div>
          <dl class="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
            <div v-for="stat in usageSummary" :key="stat.label" class="rounded-lg border border-default/60 bg-default px-4 py-3">
              <dt class="text-xs text-muted">{{ stat.label }}</dt>
              <dd class="mt-1 text-xl font-semibold">{{ stat.value }}</dd>
            </div>
          </dl>
          <div class="grid gap-4 lg:grid-cols-2">
            <section v-for="panel in usagePanels" :key="panel.label" class="overflow-hidden rounded-lg border border-default/60 bg-default">
              <div class="border-b border-default/60 bg-elevated/20 px-4 py-2.5">
                <h3 class="text-sm font-medium text-muted">{{ panel.label }}</h3>
              </div>
              <div class="p-4">
                <div class="border-b border-default/60 pb-4">
                  <div v-for="value in panel.values" :key="value.label" class="min-w-0">
                    <div class="flex items-center gap-2 text-xs text-muted">
                      <span class="size-2 rounded-full" :class="value.color" />
                      <span class="truncate">{{ value.label }}</span>
                    </div>
                    <p class="mt-1 font-medium">{{ value.value }}</p>
                  </div>
                </div>
                <div class="usage-plot mt-4 flex min-h-52 items-center justify-center rounded-md border border-dashed border-default/60 px-6 text-center">
                  <p class="text-sm text-muted">No data is available for this time range</p>
                </div>
              </div>
            </section>
          </div>
          <p class="text-xs text-muted">Usage metrics are placeholders until the storage service exposes bucket-level telemetry.</p>
        </div>
      </template>
    </UiTabs>

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
