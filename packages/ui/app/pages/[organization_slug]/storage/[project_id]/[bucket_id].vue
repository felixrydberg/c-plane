<script setup lang="ts">
import type { Bucket } from '@cplane/sdk'
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

const store = useStore()
const route = useRoute()
const toast = useToast()
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
const folders = ref<string[]>([])
const objects = ref<BucketObject[]>([])
const nextContinuationToken = ref<string | null>(null)
const loadingMore = ref(false)
const downloading = ref<string | null>(null)
const refreshing = ref(false)
const deleteTarget = ref<DeleteTarget | null>(null)
const deleteModalOpen = ref(false)
const deleting = ref(false)

watch(initialPage, (page) => {
  folders.value = page?.folders ?? []
  objects.value = page?.objects ?? []
  nextContinuationToken.value = page?.next_continuation_token ?? null
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

async function loadMore() {
  if (!nextContinuationToken.value || loadingMore.value) return
  loadingMore.value = true
  try {
    const page = await cplaneFetch<BucketObjectsPage>(objectsUrl.value, {
      query: { prefix: prefix.value || undefined, continuation_token: nextContinuationToken.value },
    })
    folders.value.push(...page.folders)
    objects.value.push(...page.objects)
    nextContinuationToken.value = page.next_continuation_token
  } catch {
    toast.add({ title: 'Failed to load more objects', color: 'error' })
  } finally {
    loadingMore.value = false
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
          <UButton :icon="ICONS.refresh" color="neutral" variant="solid" :loading="refreshing" aria-label="Reload objects" @click="reloadObjects" />
        </div>
      </div>
    </div>

    <div v-if="status === 'pending'" class="flex items-center justify-center py-14 text-sm text-muted">Loading objects…</div>
    <div v-else-if="error" class="flex flex-col items-center justify-center gap-3 py-14 text-center">
      <p class="text-sm text-muted">Unable to load this bucket.</p>
      <UButton :icon="ICONS.refresh" color="neutral" variant="solid" :loading="refreshing" aria-label="Retry loading objects" @click="reloadObjects" />
    </div>
    <div v-else-if="!folders.length && !objects.length" class="flex flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-default py-14 text-center">
      <UIcon :name="ICONS.folder" class="size-10 text-muted" />
      <p class="text-muted">This folder is empty.</p>
    </div>
    <div v-else class="overflow-hidden rounded-lg border border-default/60">
      <table class="w-full text-sm">
        <thead class="bg-elevated text-left">
          <tr><th class="p-3">Name</th><th class="hidden p-3 sm:table-cell">Modified</th><th class="hidden p-3 text-right sm:table-cell">Size</th><th class="p-3"><span class="sr-only">Actions</span></th></tr>
        </thead>
        <tbody>
          <tr v-for="folder in folders" :key="folder" class="border-t border-default/60">
            <td colspan="4" class="p-0">
              <div class="flex items-center gap-2 p-2 hover:bg-elevated">
                <button class="flex min-w-0 flex-1 items-center gap-2 p-1 text-left" @click="openPrefix(folder)">
                  <UIcon :name="ICONS.folder" class="size-4 shrink-0 text-muted" />
                  <span class="truncate">{{ folder.slice(prefix.length).replace(/\/$/, '') }}</span>
                </button>
                <UButton :icon="ICONS.trash" color="error" size="xs" @click="confirmDelete('folder', folder)">Delete</UButton>
              </div>
            </td>
          </tr>
          <tr v-for="object in objects" :key="object.key" class="border-t border-default/60">
            <td class="p-3 font-mono text-xs break-all">{{ object.key.slice(prefix.length) }}</td>
            <td class="hidden p-3 text-xs text-muted sm:table-cell">{{ object.last_modified ? new Date(object.last_modified).toLocaleString() : '—' }}</td>
            <td class="hidden p-3 text-right text-xs text-muted sm:table-cell">{{ formatSize(object.size) }}</td>
            <td class="p-3 text-right">
              <div class="flex justify-end gap-2">
                <UButton :icon="ICONS.download" color="neutral" variant="solid" size="xs" :loading="downloading === object.key" @click="download(object)">Download</UButton>
                <UButton :icon="ICONS.trash" color="error" size="xs" :loading="deleting && deleteTarget?.key === object.key" @click="confirmDelete('object', object.key)">Delete</UButton>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <div v-if="nextContinuationToken" class="flex justify-center"><UButton :icon="ICONS.refresh" color="neutral" variant="solid" :loading="loadingMore" @click="loadMore">Load more</UButton></div>

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
