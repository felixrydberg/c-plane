<script setup lang="ts">
import { ICONS } from '~/utils/icons'

interface Repository { id: string; name: string }
interface RepositoryPermission { repository_id: string; can_pull: boolean; can_push: boolean }
interface AccessToken { name: string; repository_permissions: RepositoryPermission[] }

const store = useStore()
const route = useRoute()
const toast = useToast()
const organizationId = computed(() => store.organization?.id ?? '')
const tokenId = computed(() => route.params.token_id?.toString() ?? '')
const grants = ref<Record<string, RepositoryPermission>>({})
const loading = ref(false)
const repositoriesUrl = computed(() => organizationId.value
  ? `/api/backend/organization/${organizationId.value}/registry/repositories`
  : '')
const tokenUrl = computed(() => organizationId.value && tokenId.value
  ? `/api/backend/organization/${organizationId.value}/registry/access-tokens/${tokenId.value}`
  : '')
const [{ data: repositories }, { data: token }] = await Promise.all([
  useFetch<Repository[]>(repositoriesUrl, { default: () => [] }),
  useFetch<AccessToken>(tokenUrl),
])

watch([repositories, token], ([items, accessToken]) => {
  if (!accessToken) return
  const existing = Object.fromEntries(accessToken.repository_permissions.map(permission => [permission.repository_id, permission]))
  grants.value = Object.fromEntries(items.map(repository => [repository.id, existing[repository.id] ?? { repository_id: repository.id, can_pull: false, can_push: false }]))
}, { immediate: true })

const selectedPermissions = computed(() => Object.values(grants.value).filter(permission => permission.can_pull || permission.can_push))

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

async function save() {
  if (!tokenUrl.value || !selectedPermissions.value.length) return
  loading.value = true
  try {
    await $fetch(tokenUrl.value, { method: 'PATCH', body: { repository_permissions: selectedPermissions.value } })
    toast.add({ title: 'Access token updated', color: 'success' })
    await navigateTo(backUrl())
  } catch {
    toast.add({ title: 'Failed to update access token', color: 'error' })
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="w-full max-w-[1200px] mx-auto">
    <header class="border-b border-default/60 pb-5">
      <UiBackLink label="Access tokens" :to="backUrl()" />
      <h1 class="mt-2 text-2xl font-semibold">Edit Access Token</h1>
      <p class="mt-1 text-sm text-muted">Update the repositories this token can access.</p>
    </header>

    <div class="grid lg:grid-cols-[minmax(0,1fr)_280px]">
      <main class="lg:pr-8">
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
      </main>

      <aside class="border-t border-default/60 py-8 lg:border-l lg:border-t-0 lg:pl-6">
        <div class="sticky top-6 rounded-lg border border-dashed border-default p-5">
          <h2 class="text-sm font-semibold">Token Summary</h2>
          <dl class="mt-5 space-y-4 text-sm">
            <div><dt class="text-xs text-muted">Name</dt><dd class="mt-1">{{ token?.name }}</dd></div>
            <div><dt class="text-xs text-muted">Repositories</dt><dd class="mt-1">{{ selectedPermissions.length }} granted</dd></div>
          </dl>
          <div class="mt-8 flex gap-3">
            <UButton :icon="ICONS.xMark" color="neutral" variant="ghost" :to="backUrl()">Cancel</UButton>
            <UButton :icon="ICONS.check" color="primary" :loading="loading" :disabled="!selectedPermissions.length" @click="save">Save</UButton>
          </div>
        </div>
      </aside>
    </div>
  </div>
</template>
