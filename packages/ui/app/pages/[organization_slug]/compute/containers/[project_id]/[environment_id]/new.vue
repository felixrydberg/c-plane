<script setup lang="ts">
import type { Region } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'
import { CPU_PRESETS, MEMORY_PRESETS_MIB, formatMemoryMib, nearestPreset } from '~/utils/compute-units'
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
  ? `/api/organization/${orgId.value as ':organization_id'}/registry/external-registries` as const
  : '')
const { data: externalRegistries } = await useCplaneFetch(externalRegistriesUrl, { default: () => [] })
const externalRegistryId = ref('none')
const externalRegistryItems = computed(() => [
  { label: 'No external registry', value: 'none' },
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
  cpu: 0.5,
  memoryMib: 1024,
  isPublic: false,
  healthCheckPath: '/health',
})

const cpuIndex = computed({
  get: () => Math.max(0, CPU_PRESETS.indexOf(nearestPreset(CPU_PRESETS, state.cpu))),
  set: (i: number) => { state.cpu = CPU_PRESETS[i] ?? CPU_PRESETS[0] ?? 0.5 },
})
const memoryIndex = computed({
  get: () => Math.max(0, MEMORY_PRESETS_MIB.indexOf(nearestPreset(MEMORY_PRESETS_MIB, state.memoryMib))),
  set: (i: number) => { state.memoryMib = MEMORY_PRESETS_MIB[i] ?? MEMORY_PRESETS_MIB[0] ?? 1024 },
})

const regions = ref<Region[]>([])
const regionId = ref('')

onMounted(async () => {
  if (!orgId.value) return
  try { regions.value = await cplaneFetch(`/api/organization/${orgId.value as ':organization_id'}/regions` as const) } catch { regions.value = [] }
})

interface EnvRow { key: string; value: string }
const envRows = ref<EnvRow[]>([])

function addEnvRow() { envRows.value.push({ key: '', value: '' }) }
function removeEnvRow(i: number) { envRows.value.splice(i, 1) }

async function handleCreate() {
  if (!orgId.value || !projectId.value || !regionId.value || !environmentId.value) return
  loading.value = true; error.value = ''
  try {
    const envObj: Record<string, string> = {}
    for (const row of envRows.value) { const k = row.key.trim(); if (k) envObj[k] = row.value }

    const body: Record<string, unknown> = {
      name: state.name.trim(), image: state.image.trim(), project_id: projectId.value, environment_id: environmentId.value,
      port: state.port, replica_count: state.replicas, public: state.isPublic,
      health_check: { path: state.healthCheckPath }, region_id: regionId.value,
      cpu: String(state.cpu),
      memory: `${Math.round(state.memoryMib)}Mi`,
      auto_deploy: false,
      external_registry_id: externalRegistryId.value === 'none' ? null : externalRegistryId.value,
    }
    if (Object.keys(envObj).length > 0) body.env = envObj

    await cplaneFetch(`/api/organization/${orgId.value as ':organization_id'}/containers` as const, { method: 'POST', body })
    await loadProjectEnvironments(projectId.value, environmentId.value)
    toast.add({ title: 'Container added', description: 'Review this environment to deploy it.', color: 'success' })
    const path = `/${route.params.organization_slug}/compute/containers/${projectId.value}/${environmentId.value}`
    navigateTo(path)
  } catch (e: unknown) {
    error.value = getErrorMessage(e, 'Failed to create container')
    const message = getErrorMessage(e, '')
    toast.add({ title: 'Failed to create container', description: message, color: 'error' })
  } finally { loading.value = false }
}

function backUrl() { return `/${route.params.organization_slug}/compute/containers/${projectId.value}/${environmentId.value}` }
</script>

<template>
  <div class="w-full max-w-[1280px] mx-auto">
    <header class="border-b border-default/60 pb-5">
      <UiBackLink :label="projectName" :to="backUrl()" />
      <UiPageEyebrow label="Compute" />
      <h1 class="mt-2 text-2xl font-semibold">Add container</h1>
      <p class="mt-1 text-sm text-muted">Configure a service, then review it with the rest of this environment before deployment.</p>
    </header>

    <div class="grid gap-0 lg:grid-cols-[minmax(0,1fr)_280px]">
      <main class="divide-y divide-default/60 lg:pr-8">
        <section class="grid gap-4 py-7 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Container</h2><p class="mt-1 text-xs text-muted">Name the service and choose its image.</p></div>
          <div class="grid gap-3"><UFormField label="Name"><UInput v-model="state.name" placeholder="api-gateway" class="w-full" :disabled="loading" /></UFormField><UFormField label="Image"><UInput v-model="state.image" placeholder="nginx:latest" class="w-full" :disabled="loading" /></UFormField><UFormField label="External registry" description="Optional credentials for a private image."><USelect v-model="externalRegistryId" :items="externalRegistryItems" class="w-full" :disabled="loading" /></UFormField></div>
        </section>
        <section class="grid gap-4 py-7 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Compute</h2><p class="mt-1 text-xs text-muted">CPU, memory, network, and scale.</p></div>
          <div class="grid gap-3 sm:grid-cols-2"><UFormField :label="`CPU · ${state.cpu} cores`" description="Preset cores per replica."><USlider v-model="cpuIndex" :min="0" :max="CPU_PRESETS.length - 1" :step="1" class="py-1" :ui="{ range: 'transition-all duration-150 ease-out', thumb: 'transition-all duration-150 ease-out' }" /></UFormField><UFormField :label="`Memory · ${formatMemoryMib(state.memoryMib)}`" description="Preset memory per replica."><USlider v-model="memoryIndex" :min="0" :max="MEMORY_PRESETS_MIB.length - 1" :step="1" class="py-1" :ui="{ range: 'transition-all duration-150 ease-out', thumb: 'transition-all duration-150 ease-out' }" /></UFormField><UFormField label="Port" description="The container port your app listens on (1–65535)."><UInput v-model.number="state.port" type="number" :min="1" :max="65535" class="w-full" /></UFormField><UFormField label="Replicas" description="Number of container instances to run."><UInputNumber v-model="state.replicas" :min="1" :step="1" class="w-full" /></UFormField></div>
        </section>
        <section class="grid gap-4 py-7 lg:grid-cols-[190px_minmax(0,1fr)]">
          <div><h2 class="text-sm font-semibold">Deployment</h2><p class="mt-1 text-xs text-muted">Placement, health, and visibility.</p></div>
          <div class="grid gap-4"><UFormField label="Region" description="Where your container runs. Pick the region closest to your users."><USelect v-model="regionId" :items="regions.map(r => ({ label: r.display_name, value: r.id }))" placeholder="Select a region" class="w-full" /></UFormField><UFormField label="Health check" description="Path your app exposes for liveness probes (e.g. /health)."><UInput v-model="state.healthCheckPath" placeholder="/health" class="w-full" /></UFormField><UFormField label="Endpoint visibility"><div class="mt-1 grid grid-cols-2 gap-2"><button type="button" class="flex flex-col items-start gap-0.5 rounded-md border-2 p-3 text-left transition-colors" :class="!state.isPublic ? 'border-primary bg-primary/10' : 'border-default/40 hover:border-default/60'" @click="state.isPublic = false"><span class="text-sm font-semibold">Private</span><span class="text-xs text-muted">Internal network only</span></button><button type="button" class="flex flex-col items-start gap-0.5 rounded-md border-2 p-3 text-left transition-colors" :class="state.isPublic ? 'border-primary bg-primary/10' : 'border-default/40 hover:border-default/60'" @click="state.isPublic = true"><span class="text-sm font-semibold">Public</span><span class="text-xs text-muted">Accessible from the web</span></button></div></UFormField></div>
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
            <UButton size="sm" color="neutral" :icon="ICONS.plus" @click="addEnvRow">Add Variable</UButton>
          </div>
        </section>
        <p v-if="error" class="py-4 text-sm text-error">{{ error }}</p>
      </main>

      <aside class="border-t border-default/60 py-7 lg:border-l lg:border-t-0 lg:pl-6">
        <div class="sticky top-6 rounded-lg border border-dashed border-default p-5">
          <h2 class="text-sm font-semibold">Pending change</h2>
          <dl class="mt-5 space-y-4 text-sm">
            <div><dt class="text-xs text-muted">Project</dt><dd class="mt-1">{{ projectName }}</dd></div>
            <div><dt class="text-xs text-muted">Image</dt><dd class="mt-1 truncate font-mono text-xs">{{ state.image || 'Not set' }}</dd></div>
            <div><dt class="text-xs text-muted">Compute</dt><dd class="mt-1">{{ state.cpu }} CPU · {{ formatMemoryMib(state.memoryMib) }}</dd></div>
            <div><dt class="text-xs text-muted">Exposure</dt><dd class="mt-1">{{ state.isPublic ? 'Public' : 'Private' }}</dd></div>
          </dl>
          <p class="mt-8 text-xs text-muted">This container will be saved with the environment's pending changes. Nothing goes live until you deploy the release.</p>
          <div class="mt-5 flex gap-3">
            <UButton variant="ghost" color="neutral" :to="backUrl()">Cancel</UButton>
            <UButton :icon="ICONS.plus" color="primary" :loading="loading" :disabled="!state.name.trim() || !state.image.trim() || !regionId" @click="handleCreate">Add container</UButton>
          </div>
        </div>
      </aside>
    </div>
  </div>
</template>
