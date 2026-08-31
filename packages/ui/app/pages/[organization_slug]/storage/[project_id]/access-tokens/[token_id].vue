<script setup lang="ts">
import type { BucketPermission } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const toast = useToast()
const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() ?? '')
const tokenId = computed(() => route.params.token_id?.toString() ?? '')
const grants = ref<Record<string, BucketPermission>>({})
const loading = ref(false)
const bucketsUrl = computed(() => orgId.value && projectId.value
  ? `/api/organization/${orgId.value as ':organization_id'}/storage/buckets` as const
  : '')
const tokenUrl = computed(() => orgId.value && projectId.value && tokenId.value
  ? `/api/organization/${orgId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/storage/access-tokens/${tokenId.value as ':token_id'}` as const
  : '')
const [{ data: buckets }, { data: token }] = await Promise.all([
  useCplaneFetch(bucketsUrl, { default: () => [], query: { project_id: projectId } }),
  useCplaneFetch(tokenUrl),
])

watch([buckets, token], ([items, accessToken]) => {
  if (!accessToken) return
  const existing = Object.fromEntries(accessToken.bucket_permissions.map(permission => [permission.bucket_id, permission]))
  grants.value = Object.fromEntries(items.map(bucket => [bucket.id, existing[bucket.id] ?? { bucket_id: bucket.id, can_read: false, can_write: false }]))
}, { immediate: true })

const selectedPermissions = computed(() => Object.values(grants.value).filter(permission => permission.can_read || permission.can_write))
function backUrl() { return `/${route.params.organization_slug}/storage/${projectId.value}/access-tokens` }
function setRead(bucketId: string, canRead: boolean) { if (grants.value[bucketId]) grants.value[bucketId].can_read = canRead }
function setWrite(bucketId: string, canWrite: boolean) { if (grants.value[bucketId]) grants.value[bucketId].can_write = canWrite }

async function save() {
  if (!tokenUrl.value || !selectedPermissions.value.length) return
  loading.value = true
  try {
    await cplaneFetch(tokenUrl.value, { method: 'PATCH', body: { bucket_permissions: selectedPermissions.value } })
    toast.add({ title: 'Access token updated', color: 'success' })
    await navigateTo(backUrl())
  } catch {
    toast.add({ title: 'Failed to update access token', color: 'error' })
  } finally { loading.value = false }
}
</script>

<template>
  <div class="w-full max-w-[1200px] mx-auto">
    <header class="border-b border-default/60 pb-5">
      <UiBackLink label="Access tokens" :to="backUrl()" />
      <h1 class="mt-2 text-2xl font-semibold">Edit Access Token</h1>
      <p class="mt-1 text-sm text-muted">Update the buckets this token can access.</p>
    </header>

    <div class="grid lg:grid-cols-[minmax(0,1fr)_280px]">
      <main class="divide-y divide-default/60 lg:pr-8">
        <section class="grid gap-4 py-8 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Token</h2><p class="mt-1 text-xs text-muted">The access key and object prefix cannot be changed.</p></div>
          <div class="space-y-4"><UFormField label="Access key ID"><UInput :model-value="token?.access_key_id" readonly class="w-full font-mono" /></UFormField><UFormField label="Object key prefix"><UInput :model-value="token?.prefix" readonly class="w-full font-mono" /></UFormField></div>
        </section>
        <section class="grid gap-4 py-8 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Bucket Permissions</h2><p class="mt-1 text-xs text-muted">Choose what this token can do in each project bucket.</p></div>
          <div class="overflow-hidden rounded-lg border border-default/60"><table class="w-full text-sm"><thead class="bg-elevated text-left"><tr><th class="p-3">Bucket</th><th class="p-3 text-center">Read</th><th class="p-3 text-center">Write</th></tr></thead><tbody><tr v-for="bucket in buckets" :key="bucket.id" class="border-t border-default/60"><td class="p-3 font-medium">{{ bucket.name }}</td><td class="p-3"><div class="flex justify-center"><USwitch :model-value="grants[bucket.id]?.can_read" :disabled="loading" :aria-label="`Read ${bucket.name}`" @update:model-value="setRead(bucket.id, Boolean($event))" /></div></td><td class="p-3"><div class="flex justify-center"><USwitch :model-value="grants[bucket.id]?.can_write" :disabled="loading" :aria-label="`Write ${bucket.name}`" @update:model-value="setWrite(bucket.id, Boolean($event))" /></div></td></tr></tbody></table></div>
        </section>
      </main>

      <aside class="border-t border-default/60 py-8 lg:border-l lg:border-t-0 lg:pl-6">
        <div class="sticky top-6"><h2 class="text-sm font-semibold">Token Summary</h2><dl class="mt-5 space-y-4 text-sm"><div><dt class="text-xs text-muted">Name</dt><dd class="mt-1">{{ token?.name }}</dd></div><div><dt class="text-xs text-muted">Buckets</dt><dd class="mt-1">{{ selectedPermissions.length }} granted</dd></div></dl><div class="mt-8 flex gap-3"><UButton color="neutral" variant="ghost" :to="backUrl()">Cancel</UButton><UButton :icon="ICONS.check" color="primary" :loading="loading" :disabled="!selectedPermissions.length" @click="save">Save</UButton></div></div>
      </aside>
    </div>
  </div>
</template>
