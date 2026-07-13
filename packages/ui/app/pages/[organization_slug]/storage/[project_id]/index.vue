<script setup lang="ts">
import { ICONS } from '~/utils/icons'

interface Bucket { id: string; name: string; region: string; is_public: boolean; status: string }
interface Branch { id: string; name: string }

const store = useStore()
const route = useRoute()
const organizationId = computed(() => store.organization?.id || '')
const projectId = computed(() => route.params.project_id as string)
const projectName = computed(() => store.projects.find(project => project.id === projectId.value)?.name ?? projectId.value)
const bucketsUrl = computed(() => organizationId.value ? `/api/backend/organization/${organizationId.value}/storage/buckets?project_id=${projectId.value}` : '')
const branchesUrl = computed(() => organizationId.value ? `/api/backend/organization/${organizationId.value}/projects/${projectId.value}/branches` : '')
const [{ data: buckets }, { data: branches }] = await Promise.all([
  useFetch<Bucket[]>(bucketsUrl, { default: () => [] }),
  useFetch<Branch[]>(branchesUrl, { default: () => [] }),
])
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div class="flex items-start justify-between gap-4">
      <div><h1 class="text-2xl font-semibold">Storage</h1><p class="text-muted text-sm mt-1">Buckets and branch views for {{ projectName }}.</p></div>
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
        <div><h2 class="font-semibold">{{ bucket.name }}</h2><p class="text-muted text-sm">{{ bucket.is_public ? 'Public' : 'Private' }} · {{ bucket.status }}</p></div>
        <span class="text-sm text-muted">{{ branches.length }} branch{{ branches.length === 1 ? '' : 'es' }}</span>
      </div>
      <div v-if="branches.length" class="divide-y divide-default">
        <div v-for="branch in branches" :key="branch.id" class="flex items-center justify-between gap-3 p-4">
          <div><p class="font-medium text-sm">{{ branch.name }}</p><p class="text-muted text-xs">Bucket branch view</p></div>
        </div>
      </div>
      <div v-else class="p-4 text-sm text-muted">No project branches.</div>
    </section>
  </div>
</template>
