<script setup lang="ts">
import { ICONS } from '~/utils/icons'

interface Repository { id: string; name: string }
interface RepositoryPermission { repository_id: string; can_pull: boolean; can_push: boolean }
interface AccessToken { name: string; token: string }

const store = useStore()
const route = useRoute()
const toast = useToast()
const config = useRuntimeConfig()
const organizationId = computed(() => store.organization?.id ?? '')
const organizationSlug = computed(() => store.organization?.slug ?? '')
const registryHost = computed(() => config.public.registryHost)
const name = ref('')
const grants = ref<Record<string, RepositoryPermission>>({})
const loading = ref(false)
const error = ref('')
const created = ref<AccessToken | null>(null)
const repositoriesUrl = computed(() => organizationId.value
  ? `/api/backend/organization/${organizationId.value}/registry/repositories`
  : '')
const { data: repositories } = await useFetch<Repository[]>(repositoriesUrl, { default: () => [] })

watch(repositories, (items) => {
  for (const repository of items) {
    grants.value[repository.id] ??= { repository_id: repository.id, can_pull: false, can_push: false }
  }
}, { immediate: true })

const selectedPermissions = computed(() => Object.values(grants.value).filter(permission => permission.can_pull || permission.can_push))
const loginCommand = computed(() => `echo "$CPLANE_REGISTRY_TOKEN" | docker login "${registryHost.value}" --username "${organizationSlug.value}" --password-stdin`)

function backUrl() { return `/${route.params.organization_slug}/registry/access-tokens` }
function setPull(repositoryId: string, canPull: boolean) {
  const grant = grants.value[repositoryId]
  if (!grant) return
  grant.can_pull = canPull
  if (!canPull) grant.can_push = false
}
function setPush(repositoryId: string, canPush: boolean) {
  const grant = grants.value[repositoryId]
  if (!grant) return
  grant.can_push = canPush
  if (canPush) grant.can_pull = true
}

async function createToken() {
  if (!organizationId.value || !name.value.trim() || !selectedPermissions.value.length) return
  loading.value = true
  error.value = ''
  try {
    created.value = await $fetch<AccessToken>(`/api/backend/organization/${organizationId.value}/registry/access-tokens`, {
      method: 'POST',
      body: { name: name.value.trim(), repository_permissions: selectedPermissions.value },
    })
    toast.add({ title: 'Access token created', color: 'success' })
  } catch (cause: unknown) {
    error.value = cause instanceof Error ? cause.message : 'Failed to create access token'
    toast.add({ title: 'Failed to create access token', color: 'error' })
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="w-full max-w-[1200px] mx-auto">
    <header class="border-b border-default/60 pb-5">
      <UiBackLink label="Access tokens" :to="backUrl()" />
      <h1 class="mt-2 text-2xl font-semibold">{{ created ? 'Save Access Token' : 'New Access Token' }}</h1>
      <p class="mt-1 text-sm text-muted">{{ created ? 'Copy the token now. It will not be shown again.' : 'Create credentials for container clients and CI pipelines.' }}</p>
    </header>

    <section v-if="created" class="space-y-5 py-8 max-w-3xl">
      <p class="rounded-lg border border-warning/40 bg-warning/10 p-4 text-sm">Store the token in a password manager or CI secret before leaving this page.</p>
      <UFormField label="Registry"><UInput :model-value="registryHost" readonly class="w-full font-mono" /></UFormField>
      <UFormField label="Username"><UInput :model-value="organizationSlug" readonly class="w-full font-mono" /></UFormField>
      <UFormField label="Access token"><UInput :model-value="created.token" readonly class="w-full font-mono" /></UFormField>
      <UFormField label="Docker login"><UTextarea :model-value="loginCommand" readonly autoresize class="w-full font-mono text-xs" /></UFormField>
      <UButton :icon="ICONS.check" color="primary" :to="backUrl()">I Saved the Token</UButton>
    </section>

    <div v-else class="grid lg:grid-cols-[minmax(0,1fr)_280px]">
      <main class="divide-y divide-default/60 lg:pr-8">
        <section class="grid gap-4 py-8 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Token</h2><p class="mt-1 text-xs text-muted">Name this client so you can identify it later.</p></div>
          <UFormField label="Name"><UInput v-model="name" placeholder="production-deploy" :disabled="loading" /></UFormField>
        </section>
        <section class="grid gap-4 py-8 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Repository Permissions</h2><p class="mt-1 text-xs text-muted">Choose what this token can do in each organization repository.</p></div>
          <div class="overflow-hidden rounded-lg border border-default/60">
            <table class="w-full text-sm">
              <thead class="bg-elevated text-left"><tr><th class="p-3">Repository</th><th class="p-3 text-center">Pull</th><th class="p-3 text-center">Push</th></tr></thead>
              <tbody>
                <tr v-if="!repositories.length"><td colspan="3" class="p-6 text-center text-muted">No repositories in this organization.</td></tr>
                <tr v-for="repository in repositories" :key="repository.id" class="border-t border-default/60">
                  <td class="p-3 font-medium">{{ repository.name }}</td>
                  <td class="p-3"><div class="flex justify-center"><USwitch :model-value="grants[repository.id]?.can_pull" :disabled="loading" :aria-label="`Pull ${repository.name}`" @update:model-value="setPull(repository.id, Boolean($event))" /></div></td>
                  <td class="p-3"><div class="flex justify-center"><USwitch :model-value="grants[repository.id]?.can_push" :disabled="loading" :aria-label="`Push ${repository.name}`" @update:model-value="setPush(repository.id, Boolean($event))" /></div></td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>
        <p v-if="error" class="py-4 text-sm text-error">{{ error }}</p>
      </main>

      <aside class="border-t border-default/60 py-8 lg:border-l lg:border-t-0 lg:pl-6">
        <div class="sticky top-6 rounded-lg border border-dashed border-default p-5">
          <h2 class="text-sm font-semibold">Token Summary</h2>
          <dl class="mt-5 space-y-4 text-sm">
            <div><dt class="text-xs text-muted">Organization</dt><dd class="mt-1">{{ organizationSlug }}</dd></div>
            <div><dt class="text-xs text-muted">Name</dt><dd class="mt-1 font-mono text-xs">{{ name || 'Not set' }}</dd></div>
            <div><dt class="text-xs text-muted">Repositories</dt><dd class="mt-1">{{ selectedPermissions.length }} granted</dd></div>
          </dl>
          <div class="mt-8 flex gap-3">
            <UButton :icon="ICONS.xMark" color="neutral" variant="ghost" :to="backUrl()">Cancel</UButton>
            <UButton :icon="ICONS.plus" color="primary" :loading="loading" :disabled="!name.trim() || !selectedPermissions.length" @click="createToken">Create Access Token</UButton>
          </div>
        </div>
      </aside>
    </div>
  </div>
</template>
