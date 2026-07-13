<script setup lang="ts">
interface Region { id: string; display_name: string }

const store = useStore()
const route = useRoute()
const toast = useToast()
const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() ?? '')
const projectName = computed(() => store.projects.find(project => project.id === projectId.value)?.name ?? projectId.value)
const name = ref('')
const regionId = ref('')
const isPublic = ref(false)
const loading = ref(false)
const error = ref('')
const regionsUrl = computed(() => orgId.value ? `/api/backend/organization/${orgId.value}/regions` : '')
const { data: regions } = await useFetch<Region[]>(regionsUrl, { default: () => [] })

function backUrl() { return `/${route.params.organization_slug}/storage/${projectId.value}` }

async function createBucket() {
  if (!orgId.value || !projectId.value || !name.value.trim() || !regionId.value) return
  loading.value = true
  error.value = ''
  try {
    await $fetch(`/api/backend/organization/${orgId.value}/storage/buckets`, {
      method: 'POST',
      body: { project_id: projectId.value, name: name.value.trim(), region: regionId.value, is_public: isPublic.value },
    })
    toast.add({ title: 'Bucket created', color: 'success' })
    await navigateTo(backUrl())
  } catch (cause: unknown) {
    error.value = cause instanceof Error ? cause.message : 'Failed to create bucket'
    toast.add({ title: 'Failed to create bucket', color: 'error' })
  } finally { loading.value = false }
}
</script>

<template>
  <div class="w-full max-w-[1200px] mx-auto">
    <header class="border-b border-default/60 pb-5">
      <UiBackLink :label="projectName" :to="backUrl()" />
      <h1 class="mt-2 text-2xl font-semibold">New Bucket</h1>
      <p class="mt-1 text-sm text-muted">Create storage shared by every branch in this project.</p>
    </header>

    <div class="grid lg:grid-cols-[minmax(0,1fr)_280px]">
      <main class="divide-y divide-default/60 lg:pr-8">
        <section class="grid gap-4 py-8 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Bucket</h2><p class="mt-1 text-xs text-muted">Names are global and cannot be changed.</p></div>
          <UFormField label="Name" description="3–63 lowercase letters, numbers, dots, and hyphens."><UInput v-model="name" placeholder="media-assets" class="w-full" :disabled="loading" /></UFormField>
        </section>
        <section class="grid gap-4 py-8 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Placement</h2><p class="mt-1 text-xs text-muted">Choose where objects are stored.</p></div>
          <UFormField label="Region"><USelect v-model="regionId" :items="regions.map(region => ({ label: region.display_name, value: region.id }))" placeholder="Select a region" class="w-full" :disabled="loading" /></UFormField>
        </section>
        <section class="grid gap-4 py-8 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Visibility</h2><p class="mt-1 text-xs text-muted">Control how objects can be served.</p></div>
          <div class="grid grid-cols-2 gap-2"><button type="button" class="flex flex-col items-start gap-0.5 rounded-lg border-2 p-3 text-left transition-colors" :class="!isPublic ? 'border-primary bg-primary/10' : 'border-default/40 hover:border-default/60'" @click="isPublic = false"><span class="text-sm font-semibold">Private</span><span class="text-xs text-muted">Authenticated access only</span></button><button type="button" class="flex flex-col items-start gap-0.5 rounded-lg border-2 p-3 text-left transition-colors" :class="isPublic ? 'border-primary bg-primary/10' : 'border-default/40 hover:border-default/60'" @click="isPublic = true"><span class="text-sm font-semibold">Public</span><span class="text-xs text-muted">Allow public object delivery</span></button></div>
        </section>
        <p v-if="error" class="py-4 text-sm text-error">{{ error }}</p>
      </main>

      <aside class="border-t border-default/60 py-8 lg:border-l lg:border-t-0 lg:pl-6">
        <div class="sticky top-6"><h2 class="text-sm font-semibold">Bucket Summary</h2><dl class="mt-5 space-y-4 text-sm"><div><dt class="text-xs text-muted">Project</dt><dd class="mt-1">{{ projectName }}</dd></div><div><dt class="text-xs text-muted">Name</dt><dd class="mt-1 font-mono text-xs">{{ name || 'Not set' }}</dd></div><div><dt class="text-xs text-muted">Visibility</dt><dd class="mt-1">{{ isPublic ? 'Public' : 'Private' }}</dd></div></dl><div class="mt-8 flex gap-3"><UButton variant="ghost" color="neutral" :to="backUrl()">Cancel</UButton><UButton color="primary" :loading="loading" :disabled="!name.trim() || !regionId" @click="createBucket">Create Bucket</UButton></div></div>
      </aside>
    </div>
  </div>
</template>
