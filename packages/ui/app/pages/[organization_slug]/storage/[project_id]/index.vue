<script setup lang="ts">
import type { Bucket } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const toast = useToast()
const isOwner = computed(() => store.organization?.member?.role === 'owner')
const organizationId = computed(() => store.organization?.id || '')
const projectId = computed(() => route.params.project_id as string)
const projectName = computed(() => store.projects.find(project => project.id === projectId.value)?.name ?? projectId.value)
const bucketsUrl = computed(() => organizationId.value
  ? `/api/cplane/organization/${organizationId.value as ':organization_id'}/storage/buckets` as const
  : '')
const selectedBucket = ref<Bucket | null>(null)
const deleteModalOpen = ref(false)
const deleting = ref(false)
const { data: buckets, refresh: refreshBuckets } = await useFetch(bucketsUrl, {
  default: () => [],
  query: { project_id: projectId },
})

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
</script>

<template>
  <div class="flex w-full max-w-[1500px] flex-col gap-5 mx-auto">
    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div><h1 class="text-2xl font-semibold">Storage</h1><p class="text-muted text-sm mt-1">Buckets for {{ projectName }}.</p></div>
      <div class="flex flex-wrap justify-end gap-2">
        <UButton :icon="ICONS.authentication" color="neutral" variant="solid" :to="`/${route.params.organization_slug}/storage/${projectId}/access-tokens`">Manage access tokens</UButton>
        <UButton v-if="isOwner" :icon="ICONS.plus" color="primary" :to="`/${route.params.organization_slug}/storage/${projectId}/new`">New bucket</UButton>
      </div>
    </div>
    <div v-if="!buckets.length" class="flex flex-col items-center justify-center py-14 gap-3 text-center rounded-lg border border-dashed border-default bg-transparent">
      <UIcon :name="ICONS.storage" class="size-10 text-muted" /><p class="text-muted">No buckets yet.</p><p class="text-dimmed text-sm">Create your first bucket to start storing objects.</p>
    </div>
    <section v-for="bucket in buckets" :key="bucket.id" class="overflow-hidden rounded-lg border border-dashed border-default bg-transparent">
      <div class="flex items-center justify-between gap-3 p-4 border-b border-default">
        <div><h2 class="font-semibold">{{ bucket.name }}</h2></div>
        <div class="flex gap-2">
          <UButton :icon="ICONS.folder" color="neutral" variant="solid" size="sm" :to="`/${route.params.organization_slug}/storage/${projectId}/${bucket.id}`">View objects</UButton>
          <UButton v-if="isOwner" :icon="ICONS.trash" color="error" size="sm" @click="confirmDelete(bucket)">Delete</UButton>
        </div>
      </div>
    </section>
    <UModal v-if="isOwner" v-model:open="deleteModalOpen" title="Delete bucket" description="This deletes the physical provider bucket and its logical record. The bucket must be empty.">
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
