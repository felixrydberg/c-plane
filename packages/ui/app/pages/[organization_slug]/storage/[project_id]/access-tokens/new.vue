<script setup lang="ts">
import type { BucketPermission, CreatedStorageAccessToken } from '@cplane/sdk'
import { getErrorMessage } from '~/utils/errors'

const store = useStore()
const route = useRoute()
const toast = useToast()
const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() ?? '')
const projectName = computed(() => store.projects.find(project => project.id === projectId.value)?.name ?? projectId.value)
const name = ref('')
const grants = ref<Record<string, BucketPermission>>({})
const loading = ref(false)
const error = ref('')
const created = ref<CreatedStorageAccessToken | null>(null)
const bucketsUrl = computed(() => orgId.value && projectId.value
  ? `/api/organization/${orgId.value as ':organization_id'}/storage/buckets` as const
  : '')
const { data: buckets } = await useCplaneFetch(bucketsUrl, {
  default: () => [],
  query: { project_id: projectId },
})

watch(buckets, (items) => {
  for (const bucket of items) grants.value[bucket.id] ??= { bucket_id: bucket.id, can_read: false, can_write: false }
}, { immediate: true })

const selectedPermissions = computed(() => Object.values(grants.value).filter(permission => permission.can_read || permission.can_write))

function backUrl() { return `/${route.params.organization_slug}/storage/${projectId.value}/access-tokens` }
function setRead(bucketId: string, canRead: boolean) { if (grants.value[bucketId]) grants.value[bucketId].can_read = canRead }
function setWrite(bucketId: string, canWrite: boolean) { if (grants.value[bucketId]) grants.value[bucketId].can_write = canWrite }

async function createToken() {
  if (!orgId.value || !projectId.value || !name.value.trim() || !selectedPermissions.value.length) return
  loading.value = true
  error.value = ''
  try {
    created.value = await cplaneFetch(`/api/organization/${orgId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/storage/access-tokens` as const, { method: 'POST', body: { name: name.value.trim(), bucket_permissions: selectedPermissions.value } })
    toast.add({ title: 'Access token created', color: 'success' })
  } catch (cause: unknown) {
    error.value = getErrorMessage(cause, 'Failed to create access token')
    toast.add({ title: 'Failed to create access token', color: 'error' })
  } finally { loading.value = false }
}
</script>

<template>
  <div class="w-full max-w-[1200px] mx-auto">
    <header class="border-b border-default/60 pb-5">
      <UiBackLink label="Access tokens" :to="backUrl()" />
      <h1 class="mt-2 text-2xl font-semibold">{{ created ? 'Save Access Token' : 'New Access Token' }}</h1>
      <p class="mt-1 text-sm text-muted">{{ created ? 'Copy the secret now. It will not be shown again.' : 'Create credentials for S3 clients in this project.' }}</p>
    </header>

    <section v-if="created" class="space-y-5 py-8 max-w-3xl">
      <p class="rounded-lg border border-warning/40 bg-warning/10 p-4 text-sm">Store the secret access key in a password manager before leaving this page.</p>
      <UFormField label="S3 endpoint"><UInput :model-value="created.endpoint_url" readonly class="w-full font-mono" /></UFormField>
      <UFormField label="Access key ID"><UInput :model-value="created.access_key_id" readonly class="w-full font-mono" /></UFormField>
      <UFormField label="Secret access key"><UInput :model-value="created.secret_access_key" readonly class="w-full font-mono" /></UFormField>
      <UButton color="primary" :to="backUrl()">I Saved the Secret</UButton>
    </section>

    <div v-else class="grid lg:grid-cols-[minmax(0,1fr)_280px]">
      <main class="divide-y divide-default/60 lg:pr-8">
        <section class="grid gap-4 py-8 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Token</h2><p class="mt-1 text-xs text-muted">Name this client so you can identify it later.</p></div>
          <UFormField label="Name"><UInput v-model="name" placeholder="production-deploy" :disabled="loading" /></UFormField>
        </section>
        <section class="grid gap-4 py-8 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Bucket Permissions</h2><p class="mt-1 text-xs text-muted">Choose what this token can do in each project bucket.</p></div>
          <div class="overflow-hidden rounded-lg border border-default/60"><table class="w-full text-sm"><thead class="bg-elevated text-left"><tr><th class="p-3">Bucket</th><th class="p-3 text-center">Read</th><th class="p-3 text-center">Write</th></tr></thead><tbody><tr v-if="!buckets.length"><td colspan="3" class="p-6 text-center text-muted">No buckets in this project.</td></tr><tr v-for="bucket in buckets" :key="bucket.id" class="border-t border-default/60"><td class="p-3 font-medium">{{ bucket.name }}</td><td class="p-3"><div class="flex justify-center"><USwitch :model-value="grants[bucket.id]?.can_read" :disabled="loading" :aria-label="`Read ${bucket.name}`" @update:model-value="setRead(bucket.id, Boolean($event))" /></div></td><td class="p-3"><div class="flex justify-center"><USwitch :model-value="grants[bucket.id]?.can_write" :disabled="loading" :aria-label="`Write ${bucket.name}`" @update:model-value="setWrite(bucket.id, Boolean($event))" /></div></td></tr></tbody></table></div>
        </section>
        <p v-if="error" class="py-4 text-sm text-error">{{ error }}</p>
      </main>

      <aside class="border-t border-default/60 py-8 lg:border-l lg:border-t-0 lg:pl-6">
        <div class="sticky top-6"><h2 class="text-sm font-semibold">Token Summary</h2><dl class="mt-5 space-y-4 text-sm"><div><dt class="text-xs text-muted">Project</dt><dd class="mt-1">{{ projectName }}</dd></div><div><dt class="text-xs text-muted">Name</dt><dd class="mt-1 font-mono text-xs">{{ name || 'Not set' }}</dd></div><div><dt class="text-xs text-muted">Buckets</dt><dd class="mt-1">{{ selectedPermissions.length }} granted</dd></div></dl><div class="mt-8 flex gap-3"><UButton color="neutral" variant="ghost" :to="backUrl()">Cancel</UButton><UButton color="primary" :loading="loading" :disabled="!name.trim() || !selectedPermissions.length" @click="createToken">Create Access Token</UButton></div></div>
      </aside>
    </div>
  </div>
</template>
