<script setup lang="ts">
import type { Region } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'
import { COMPUTE_UNIT_ITEMS, computeUnitByLabel } from '~/utils/compute-units'
import { getErrorMessage } from '~/utils/errors'
import { loadProjectEnvironments } from '~/utils/auth'

const store = useStore()
const route = useRoute()
const toast = useToast()

const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() || null)
const environmentId = computed(() => route.params.environment_id?.toString() || null)
const projectName = computed(() => store.projects.find(p => p.id === projectId.value)?.name ?? projectId.value ?? '')
const externalRegistriesUrl = computed(() => orgId.value
  ? `/api/cplane/organization/${orgId.value as ':organization_id'}/registry/external-registries` as const
  : '')
const { data: externalRegistries } = await useFetch(externalRegistriesUrl, { default: () => [] })
const externalRegistryId = ref('none')
const externalRegistryItems = computed(() => [
  { label: 'No managed registry', value: 'none' },
  ...externalRegistries.value.map(registry => ({
    label: `${registry.name} — ${registry.host} (${registry.username})`,
    value: registry.id,
  })),
])

const loading = ref(false)
const error = ref('')

const state = reactive({
  name: '',
  image: '',
  port: 80,
  replicas: 1,
  isPublic: false,
  healthCheckPath: '/health',
})

const computeUnit = ref('0.5')
const createAsDraft = ref(false)

const regions = ref<Region[]>([])
const regionId = ref('')

onMounted(async () => {
  if (!orgId.value) return
  try { regions.value = await $fetch(`/api/cplane/organization/${orgId.value as ':organization_id'}/regions` as const) } catch { regions.value = [] }
})

interface EnvRow { key: string; value: string }
const envRows = ref<EnvRow[]>([])

function addEnvRow() { envRows.value.push({ key: '', value: '' }) }
function removeEnvRow(i: number) { envRows.value.splice(i, 1) }

async function handleCreate() {
  if (!orgId.value || !projectId.value || !regionId.value || !environmentId.value) return
  loading.value = true; error.value = ''
  const unit = computeUnitByLabel(computeUnit.value)
  try {
    const envObj: Record<string, string> = {}
    for (const row of envRows.value) { const k = row.key.trim(); if (k) envObj[k] = row.value }

    const body: Record<string, unknown> = {
      name: state.name.trim(), image: state.image.trim(), project_id: projectId.value, environment_id: environmentId.value,
      port: state.port, replica_count: state.replicas, public: state.isPublic,
      health_check: { path: state.healthCheckPath }, region_id: regionId.value,
      resources: { cpu: { min: unit.cpu, max: unit.cpu }, memory: { min: `${Math.round(unit.ramGib * 1024)}Mi`, max: `${Math.round(unit.ramGib * 1024)}Mi` } },
      auto_deploy: !createAsDraft.value,
      external_registry_id: externalRegistryId.value === 'none' ? null : externalRegistryId.value,
    }
    if (Object.keys(envObj).length > 0) body.env = envObj

    await $fetch(`/api/cplane/organization/${orgId.value as ':organization_id'}/containers` as const, { method: 'POST', body })
    await loadProjectEnvironments(projectId.value, environmentId.value)
    toast.add({ title: createAsDraft.value ? 'Container draft created' : 'Container created and deployed', color: 'success' })
    const path = `/${route.params.organization_slug}/containers/${projectId.value}/${environmentId.value}`
    navigateTo(createAsDraft.value && store.environment?.draft_timeline
      ? { path, query: { revision: store.environment.draft_timeline } }
      : path)
  } catch (e: unknown) {
    error.value = getErrorMessage(e, 'Failed to create container')
    const message = getErrorMessage(e, '')
    toast.add({ title: 'Failed to create container', description: message, color: 'error' })
  } finally { loading.value = false }
}

function backUrl() { return `/${route.params.organization_slug}/containers/${projectId.value}/${environmentId.value}` }
</script>

<template>
  <div class="w-full max-w-[1280px] mx-auto">
    <header class="border-b border-default/60 pb-5">
      <UiBackLink :label="projectName" :to="backUrl()" />
      <h1 class="mt-2 text-2xl font-semibold">New Container</h1>
      <p class="mt-1 text-sm text-muted">Deploy a service with one continuous configuration.</p>
    </header>

    <div class="grid gap-0 lg:grid-cols-[minmax(0,1fr)_280px]">
      <main class="divide-y divide-default/60 lg:pr-8">
        <section class="grid gap-4 py-7 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Container</h2><p class="mt-1 text-xs text-muted">Name the service and choose its image.</p></div>
          <div class="grid gap-3"><UFormField label="Name"><UInput v-model="state.name" placeholder="api-gateway" class="w-full" :disabled="loading" /></UFormField><UFormField label="Image"><UInput v-model="state.image" placeholder="nginx:latest" class="w-full" :disabled="loading" /></UFormField><UFormField label="External registry" description="Optional credentials for a private image."><USelect v-model="externalRegistryId" :items="externalRegistryItems" class="w-full" :disabled="loading" /></UFormField></div>
        </section>
        <section class="grid gap-4 py-7 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Compute</h2><p class="mt-1 text-xs text-muted">CPU, memory, network, and scale.</p></div>
          <div class="grid gap-3 sm:grid-cols-3"><UFormField label="Compute Unit" description="1 CU = 1 vCPU + 2 GB RAM. Scale from 0.25 to 32 CU."><USelect v-model="computeUnit" :items="COMPUTE_UNIT_ITEMS" class="w-full" /></UFormField><UFormField label="Port" description="The container port your app listens on (1–65535)."><UInput v-model.number="state.port" type="number" :min="1" :max="65535" class="w-full" /></UFormField><UFormField label="Replicas" description="Number of container instances to run."><UInputNumber v-model="state.replicas" :min="1" :step="1" class="w-full" /></UFormField></div>
        </section>
        <section class="grid gap-4 py-7 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Deployment</h2><p class="mt-1 text-xs text-muted">Placement, health, and visibility.</p></div>
          <div class="grid gap-4"><UFormField label="Region" description="Where your container runs. Pick the region closest to your users."><USelect v-model="regionId" :items="regions.map(r => ({ label: r.display_name, value: r.id }))" placeholder="Select a region" class="w-full" /></UFormField><UFormField label="Health check" description="Path your app exposes for liveness probes (e.g. /health)."><UInput v-model="state.healthCheckPath" placeholder="/health" class="w-full" /></UFormField><UFormField label="Endpoint visibility"><div class="mt-1 grid grid-cols-2 gap-2"><button type="button" class="flex flex-col items-start gap-0.5 rounded-lg border-2 p-3 text-left transition-colors" :class="!state.isPublic ? 'border-primary bg-primary/10' : 'border-default/40 hover:border-default/60'" @click="state.isPublic = false"><span class="text-sm font-semibold">Private</span><span class="text-xs text-muted">Internal network only</span></button><button type="button" class="flex flex-col items-start gap-0.5 rounded-lg border-2 p-3 text-left transition-colors" :class="state.isPublic ? 'border-primary bg-primary/10' : 'border-default/40 hover:border-default/60'" @click="state.isPublic = true"><span class="text-sm font-semibold">Public</span><span class="text-xs text-muted">Accessible from the web</span></button></div></UFormField></div>
        </section>
        <section class="grid gap-4 py-7 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Environment</h2><p class="mt-1 text-xs text-muted">Custom environment variables.</p></div>
          <div class="space-y-3">
            <div v-for="(row, i) in envRows" :key="i" class="grid gap-2 sm:grid-cols-[140px_minmax(0,1fr)_auto]">
              <UInput v-model="row.key" placeholder="KEY" />
              <UInput v-model="row.value" placeholder="value" />
              <UButton size="xs" color="error" :icon="ICONS.trash" @click="removeEnvRow(i)">Remove</UButton>
            </div>
            <p v-if="envRows.length === 0" class="text-sm text-muted">No environment variables configured.</p>
            <UButton size="sm" variant="solid" color="neutral" :icon="ICONS.plus" @click="addEnvRow">Add Variable</UButton>
          </div>
        </section>
        <p v-if="error" class="py-4 text-sm text-error">{{ error }}</p>
      </main>

      <aside class="border-t border-default/60 py-7 lg:border-l lg:border-t-0 lg:pl-6">
        <div class="sticky top-6 rounded-lg border border-dashed border-default p-5">
          <h2 class="text-sm font-semibold">Deployment Summary</h2>
          <dl class="mt-5 space-y-4 text-sm">
            <div><dt class="text-xs text-muted">Project</dt><dd class="mt-1">{{ projectName }}</dd></div>
            <div><dt class="text-xs text-muted">Image</dt><dd class="mt-1 truncate font-mono text-xs">{{ state.image || 'Not set' }}</dd></div>
            <div><dt class="text-xs text-muted">Compute</dt><dd class="mt-1">{{ computeUnit }}</dd></div>
            <div><dt class="text-xs text-muted">Exposure</dt><dd class="mt-1">{{ state.isPublic ? 'Public' : 'Private' }}</dd></div>
          </dl>
          <UCheckbox v-model="createAsDraft" class="mt-8" label="Create as draft" description="Save this container without deploying it." />
          <div class="mt-5 flex gap-3">
            <UButton variant="ghost" color="neutral" :to="backUrl()">Cancel</UButton>
            <UButton :icon="ICONS.check" :loading="loading" :disabled="!state.name.trim() || !state.image.trim() || !regionId" @click="handleCreate">Continue</UButton>
          </div>
        </div>
      </aside>
    </div>
  </div>
</template>
