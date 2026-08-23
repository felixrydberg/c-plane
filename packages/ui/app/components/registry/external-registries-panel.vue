<script setup lang="ts">
import type { ExternalRegistry, ExternalRegistryProvider } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'
import { getErrorMessage } from '~/utils/errors'

const props = defineProps<{ organizationId: string }>()
const toast = useToast()
const isOwner = computed(() => useStore().organization?.member?.role === 'owner')
const endpoint = computed(() => `/api/cplane/organization/${props.organizationId as ':organization_id'}/registry/external-registries` as const)
const { data: registries, refresh } = await useFetch(endpoint, { default: () => [] })

const modal = ref<'create' | 'rename' | 'rotate' | 'delete' | null>(null)
const selected = ref<ExternalRegistry | null>(null)
const loading = ref(false)
const name = ref('')
const provider = ref<ExternalRegistryProvider>('docker_hub')
const host = ref('')
const username = ref('')
const token = ref('')
const providerItems: { label: string, value: ExternalRegistryProvider }[] = [
  { label: 'Docker Hub', value: 'docker_hub' },
  { label: 'GitHub Container Registry', value: 'github' },
  { label: 'GitLab Container Registry', value: 'gitlab' },
  { label: 'Google Artifact Registry', value: 'google_artifact_registry' },
  { label: 'AWS Elastic Container Registry', value: 'aws_ecr' },
]
const needsHost = computed(() => provider.value === 'google_artifact_registry' || provider.value === 'aws_ecr')
const hostPlaceholder = computed(() => provider.value === 'aws_ecr'
  ? '123456789012.dkr.ecr.eu-north-1.amazonaws.com'
  : 'europe-west1-docker.pkg.dev')

function closeModal() {
  modal.value = null
}

function openCreate() {
  selected.value = null
  name.value = ''
  provider.value = 'docker_hub'
  host.value = ''
  username.value = ''
  token.value = ''
  modal.value = 'create'
}

function openAction(action: 'rename' | 'rotate' | 'delete', registry: ExternalRegistry) {
  selected.value = registry
  name.value = registry.name
  token.value = ''
  modal.value = action
}

async function submit() {
  if (!modal.value) return
  loading.value = true
  try {
    if (modal.value === 'create') {
      await $fetch(endpoint.value, {
        method: 'POST',
        body: {
          name: name.value,
          provider: provider.value,
          host: needsHost.value ? host.value : null,
          username: username.value,
          token: token.value,
        },
      })
    } else if (modal.value === 'rename' && selected.value) {
      await $fetch(`${endpoint.value}/${selected.value.id as ':registry_id'}` as const, { method: 'PATCH', body: { name: name.value } })
    } else if (modal.value === 'rotate' && selected.value) {
      await $fetch(`${endpoint.value}/${selected.value.id as ':registry_id'}/rotate-token` as const, { method: 'POST', body: { token: token.value } })
    } else if (modal.value === 'delete' && selected.value) {
      await $fetch(`${endpoint.value}/${selected.value.id as ':registry_id'}` as const, { method: 'DELETE' })
    }
    toast.add({ title: modal.value === 'delete' ? 'External registry deleted' : 'External registry saved', color: 'success' })
    closeModal()
    token.value = ''
    await refresh()
  } catch (error) {
    const message = getErrorMessage(error, '')
    toast.add({
      title: modal.value === 'delete' && (error as { statusCode?: number }).statusCode === 409
        ? 'Registry is used by one or more container versions.'
        : message || 'Could not update external registry',
      color: 'error',
    })
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <section class="space-y-3 border-t border-default/60 pt-5">
    <div class="flex items-center justify-between gap-3">
      <div>
        <h2 class="text-lg font-semibold">External registries</h2>
        <p class="mt-1 text-sm text-muted">Reusable credentials for private OCI registries.</p>
      </div>
      <UButton v-if="isOwner" :icon="ICONS.plus" color="primary" @click="openCreate">Add registry</UButton>
    </div>

    <div v-if="!registries.length" class="rounded-lg border border-dashed border-default py-10 text-center text-sm text-muted">
      No external registries configured.
    </div>
    <div v-else class="overflow-hidden rounded-lg border border-default">
      <table class="w-full text-sm">
        <thead class="bg-elevated text-left"><tr><th class="p-3">Name</th><th class="p-3">Host</th><th class="p-3">Username</th><th class="p-3" /></tr></thead>
        <tbody>
          <tr v-for="registry in registries" :key="registry.id" class="border-t border-default">
            <td class="p-3 font-medium">{{ registry.name }}</td>
            <td class="p-3 font-mono text-xs">{{ registry.host }}</td>
            <td class="p-3">{{ registry.username }}</td>
            <td class="p-3">
              <div v-if="isOwner" class="flex justify-end gap-2">
                <UButton :icon="ICONS.pencil" color="neutral" variant="solid" size="sm" @click="openAction('rename', registry)">Rename</UButton>
                <UButton :icon="ICONS.refresh" color="neutral" variant="solid" size="sm" @click="openAction('rotate', registry)">Rotate token</UButton>
                <UButton :icon="ICONS.trash" color="error" size="sm" @click="openAction('delete', registry)">Delete</UButton>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <UModal :open="modal === 'create'" title="Add external registry" :ui="{ content: 'max-w-md' }" @update:open="!$event && closeModal()">
      <template #body>
        <form class="space-y-4" @submit.prevent="submit">
          <UFormField label="Display name" required><UInput v-model="name" class="w-full" autofocus /></UFormField>
          <UFormField label="Provider" required><USelect v-model="provider" :items="providerItems" class="w-full" /></UFormField>
          <UFormField v-if="needsHost" label="Registry host" description="Use the registry host supplied by your cloud provider." required><UInput v-model="host" :placeholder="hostPlaceholder" class="w-full" /></UFormField>
          <UFormField label="Username" required><UInput v-model="username" class="w-full" /></UFormField>
          <UFormField label="Access token" required><UInput v-model="token" type="password" class="w-full" autocomplete="new-password" /></UFormField>
          <div class="flex justify-end gap-3 pt-2">
            <UButton color="neutral" variant="ghost" :disabled="loading" @click="closeModal">Cancel</UButton>
            <UButton type="submit" :icon="ICONS.plus" color="primary" :loading="loading">Add registry</UButton>
          </div>
        </form>
      </template>
    </UModal>

    <UModal :open="modal === 'rename'" title="Rename external registry" :ui="{ content: 'max-w-md' }" @update:open="!$event && closeModal()">
      <template #body>
        <form class="space-y-4" @submit.prevent="submit">
          <UFormField label="Display name" required><UInput v-model="name" class="w-full" autofocus /></UFormField>
          <div class="flex justify-end gap-3 pt-2">
            <UButton color="neutral" variant="ghost" :disabled="loading" @click="closeModal">Cancel</UButton>
            <UButton type="submit" :icon="ICONS.check" color="primary" :loading="loading">Save name</UButton>
          </div>
        </form>
      </template>
    </UModal>

    <UModal :open="modal === 'rotate'" title="Rotate registry token" :ui="{ content: 'max-w-md' }" @update:open="!$event && closeModal()">
      <template #body>
        <form class="space-y-4" @submit.prevent="submit">
          <p class="text-sm text-muted">The current token cannot be displayed. Enter its replacement.</p>
          <UFormField label="New access token" required><UInput v-model="token" type="password" class="w-full" autocomplete="new-password" autofocus /></UFormField>
          <div class="flex justify-end gap-3 pt-2">
            <UButton color="neutral" variant="ghost" :disabled="loading" @click="closeModal">Cancel</UButton>
            <UButton type="submit" :icon="ICONS.refresh" color="primary" :loading="loading">Rotate token</UButton>
          </div>
        </form>
      </template>
    </UModal>

    <UModal :open="modal === 'delete'" title="Delete external registry" :ui="{ content: 'max-w-md' }" @update:open="!$event && closeModal()">
      <template #body>
        <form class="space-y-4" @submit.prevent="submit">
          <p class="text-sm">Delete <strong>{{ selected?.name }}</strong>? This does not delete images from the remote registry.</p>
          <div class="flex justify-end gap-3 pt-2">
            <UButton color="neutral" variant="ghost" :disabled="loading" @click="closeModal">Cancel</UButton>
            <UButton type="submit" :icon="ICONS.trash" color="error" :loading="loading">Delete registry</UButton>
          </div>
        </form>
      </template>
    </UModal>
  </section>
</template>
