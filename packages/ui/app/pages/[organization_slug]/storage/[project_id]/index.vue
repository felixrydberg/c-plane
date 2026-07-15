<script setup lang="ts">
import { ICONS } from '~/utils/icons'

interface Bucket { id: string; name: string; region: string; status: string }

const store = useStore()
const route = useRoute()
const toast = useToast()
const organizationId = computed(() => store.organization?.id || '')
const projectId = computed(() => route.params.project_id as string)
const projectName = computed(() => store.projects.find(project => project.id === projectId.value)?.name ?? projectId.value)
const bucketsUrl = computed(() => organizationId.value ? `/api/backend/organization/${organizationId.value}/storage/buckets?project_id=${projectId.value}` : '')
const selectedBucket = ref<Bucket | null>(null)
const deleteModalOpen = ref(false)
const deleting = ref(false)
const { data: buckets, refresh: refreshBuckets } = await useFetch<Bucket[]>(bucketsUrl, { default: () => [] })

function confirmDelete(bucket: Bucket) {
  selectedBucket.value = bucket
  deleteModalOpen.value = true
}

async function deleteBucket() {
  if (!selectedBucket.value || !organizationId.value) return
  deleting.value = true
  try {
    await $fetch(`/api/backend/organization/${organizationId.value}/storage/buckets/${selectedBucket.value.id}`, { method: 'DELETE' })
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
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div class="flex items-start justify-between gap-4">
      <div><h1 class="text-2xl font-semibold">Storage</h1><p class="text-muted text-sm mt-1">Buckets for {{ projectName }}.</p></div>
      <div class="flex flex-wrap justify-end gap-2">
        <UButton :icon="ICONS.authentication" color="neutral" variant="solid" :to="`/${route.params.organization_slug}/storage/${projectId}/access-tokens`">Manage access tokens</UButton>
        <UButton :icon="ICONS.plus" color="primary" :to="`/${route.params.organization_slug}/storage/${projectId}/new`">New bucket</UButton>
      </div>
    </div>
    <div v-if="!buckets.length" class="flex flex-col items-center justify-center py-14 gap-3 text-center border border-dashed border-default rounded-lg">
      <UIcon :name="ICONS.storage" class="size-10 text-muted" /><p class="text-muted">No buckets yet.</p><p class="text-dimmed text-sm">Create your first bucket to start storing objects.</p>
    </div>
    <section v-for="bucket in buckets" :key="bucket.id" class="overflow-hidden border border-default rounded-lg">
      <div class="flex items-center justify-between gap-3 p-4 border-b border-default">
        <div><h2 class="font-semibold">{{ bucket.name }}</h2></div>
        <UButton :icon="ICONS.trash" color="error" size="sm" @click="confirmDelete(bucket)">Delete</UButton>
      </div>
    </section>
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
