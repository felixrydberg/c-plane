<script setup lang="ts">
import { ICONS } from '~/utils/icons'

defineOptions({ name: 'NewRegistryRepositoryPage' })

const store = useStore()
const route = useRoute()
const toast = useToast()
const config = useRuntimeConfig()
const orgId = computed(() => store.organization?.id ?? '')
const organizationSlug = computed(() => store.organization?.slug ?? '')
const name = ref('')
const loading = ref(false)
const error = ref('')
const registryHost = computed(() => config.public.registryHost)
const repositoryReference = computed(() => `${registryHost.value}/${organizationSlug.value}/${name.value || 'repository'}`)

function backUrl() { return `/${route.params.organization_slug}/registry` }

async function createRepository() {
  if (!orgId.value || !name.value.trim()) return
  loading.value = true
  error.value = ''
  try {
    await $fetch(`/api/backend/organization/${orgId.value}/registry/repositories`, {
      method: 'POST',
      body: { name: name.value.trim() },
    })
    toast.add({ title: 'Repository created', color: 'success' })
    await navigateTo(backUrl())
  } catch (cause: unknown) {
    error.value = cause instanceof Error ? cause.message : 'Failed to create repository'
    toast.add({ title: 'Failed to create repository', color: 'error' })
  } finally { loading.value = false }
}
</script>

<template>
  <div class="w-full max-w-[1200px] mx-auto">
    <header class="border-b border-default/60 pb-5">
      <UiBackLink label="Registry" :to="backUrl()" />
      <h1 class="mt-2 text-2xl font-semibold">New Repository</h1>
      <p class="mt-1 text-sm text-muted">Create a private repository in your organization's namespace.</p>
    </header>

    <div class="grid lg:grid-cols-[minmax(0,1fr)_360px]">
      <main class="divide-y divide-default/60 lg:pr-8">
        <section class="grid gap-4 py-8 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div>
            <h2 class="text-sm font-semibold">Repository</h2>
            <p class="mt-1 text-xs text-muted">Names cannot be changed after creation.</p>
          </div>
          <UFormField label="Name" description="Lowercase letters, numbers, dots, underscores, dashes, and slashes.">
            <UInput v-model="name" placeholder="backend/api" class="w-full" :disabled="loading" />
          </UFormField>
        </section>
        <p v-if="error" class="py-4 text-sm text-error">{{ error }}</p>
      </main>

      <aside class="border-t border-default/60 py-8 lg:border-l lg:border-t-0 lg:pl-6">
        <div class="sticky top-6 rounded-lg border border-dashed border-default p-5">
          <h2 class="text-sm font-semibold">Repository Summary</h2>
          <dl class="mt-5 space-y-4 text-sm">
            <div><dt class="text-xs text-muted">Organization</dt><dd class="mt-1 truncate" :title="organizationSlug">{{ organizationSlug }}</dd></div>
            <div><dt class="text-xs text-muted">Image path</dt><dd class="mt-1 truncate font-mono text-xs" :title="repositoryReference">{{ repositoryReference }}</dd></div>
          </dl>
          <div class="mt-8 flex gap-3">
            <UButton variant="ghost" color="neutral" :to="backUrl()">Cancel</UButton>
            <UButton :icon="ICONS.plus" color="primary" :loading="loading" :disabled="!name.trim()" @click="createRepository">Create repository</UButton>
          </div>
        </div>
      </aside>
    </div>
  </div>
</template>
