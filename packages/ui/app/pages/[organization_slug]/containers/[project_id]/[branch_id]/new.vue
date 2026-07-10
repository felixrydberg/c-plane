<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const toast = useToast()

const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() || null)
const branchId = computed(() => route.params.branch_id?.toString() || null)
const projectName = computed(() => store.projects.find(p => p.id === projectId.value)?.name ?? projectId.value ?? '')

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

import { COMPUTE_UNIT_ITEMS, computeUnitByLabel } from '~/utils/compute-units'

const computeUnit = ref('XS')

const regions = ref<{ id: string; display_name: string }[]>([])
const regionId = ref('')

onMounted(async () => {
  if (!orgId.value) return
  try { regions.value = await $fetch<{ id: string; display_name: string }[]>(`/api/organization/${orgId.value}/regions`) } catch { regions.value = [] }
})

interface EnvRow { key: string; value: string; secretId: string | null }
const envRows = ref<EnvRow[]>([])

interface ProjectSecret { id: string; name: string; version_count: number }
const projectSecrets = ref<ProjectSecret[]>([])

onMounted(async () => {
  if (!orgId.value || !projectId.value) return
  try {
    const res = await $fetch(`/api/backend/organization/${orgId.value}/projects/${projectId.value}/secrets`)
    projectSecrets.value = (res?.data ?? res ?? []) as ProjectSecret[]
  } catch { projectSecrets.value = [] }
})

function addEnvRow() { envRows.value.push({ key: '', value: '', secretId: null }) }
function removeEnvRow(i: number) { envRows.value.splice(i, 1) }
function setRowSecret(i: number, secretId: string) { envRows.value[i].secretId = secretId; envRows.value[i].value = '' }
function clearRowSecret(i: number) { envRows.value[i].secretId = null }
function secretNameForId(id: string): string { return projectSecrets.value.find(s => s.id === id)?.name ?? 'Unknown' }
function buildValueMenuItems(rowIndex: number) {
  const items: DropdownMenuItem[] = [{ label: 'Custom value', icon: ICONS.pencil, onSelect: () => clearRowSecret(rowIndex) }]
  if (projectSecrets.value.length > 0) {
    items.push({ type: 'separator', label: 'Project secrets' })
    for (const s of projectSecrets.value) {
      items.push({ label: s.name, icon: s.id === envRows.value[rowIndex]?.secretId ? ICONS.check : undefined, onSelect: () => setRowSecret(rowIndex, s.id) })
    }
  }
  return items
}

async function handleCreate() {
  if (!orgId.value || !projectId.value || !regionId.value || !branchId.value) return
  loading.value = true; error.value = ''
  const unit = computeUnitByLabel(computeUnit.value)
  try {
    const envObj: Record<string, string> = {}
    const secretRefObj: Record<string, string> = {}
    for (const row of envRows.value) { const k = row.key.trim(); if (!k) continue; if (row.secretId) secretRefObj[k] = row.secretId; else envObj[k] = row.value }

    const body: Record<string, unknown> = {
      name: state.name.trim(), image: state.image.trim(), project_id: projectId.value, branch_id: branchId.value,
      port: state.port, replica_count: state.replicas, public: state.isPublic,
      health_check: { path: state.healthCheckPath }, region_id: regionId.value,
      resources: { cpu: { min: unit.cpu, max: unit.cpu }, memory: { min: `${Math.round(unit.ramGib * 1024)}Mi`, max: `${Math.round(unit.ramGib * 1024)}Mi` } },
    }
    if (Object.keys(envObj).length > 0) body.env = envObj
    if (Object.keys(secretRefObj).length > 0) body.env_secret_refs = secretRefObj

    await $fetch(`/api/backend/organization/${orgId.value}/containers`, { method: 'POST', body })
    toast.add({ title: 'Container created', color: 'success' })
    navigateTo(`/${route.params.organization_slug}/containers/${projectId.value}/${branchId.value}`)
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : 'Failed to create container'
    toast.add({ title: 'Failed to create container', color: 'error' })
  } finally { loading.value = false }
}

function backUrl() { return `/${route.params.organization_slug}/containers/${projectId.value}/${branchId.value}` }
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div>
      <UiBackLink :label="projectName" :to="backUrl()" />
      <div class="mt-2">
        <h1 class="text-2xl font-semibold">New Container</h1>
        <p class="mt-0.5 text-sm text-muted">Deploy a service to this branch.</p>
      </div>
    </div>

    <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
      <div class="border-b border-default px-4 py-3"><p class="text-sm font-semibold">Container</p><p class="mt-0.5 text-xs text-muted">Name the service and choose its image.</p></div>
      <div class="p-4 space-y-3">
        <div class="flex items-center gap-4 px-1"><span class="w-24 text-sm">Name</span><UInput v-model="state.name" placeholder="e.g. api-gateway" class="flex-1" size="sm" :disabled="loading" /></div>
        <div class="flex items-center gap-4 px-1"><span class="w-24 text-sm">Image</span><UInput v-model="state.image" placeholder="e.g. nginx:latest" class="flex-1" size="sm" :disabled="loading" /></div>
      </div>
    </div>

    <div class="grid gap-6 lg:grid-cols-2">
      <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
        <div class="border-b border-default px-4 py-3"><p class="text-sm font-semibold">Compute</p><p class="mt-0.5 text-xs text-muted">CPU and RAM scale together as a compute unit.</p></div>
        <div class="p-4">
          <div class="flex items-center gap-4 px-1"><span class="w-28 text-sm">Compute Unit</span><div class="w-28"><USelect v-model="computeUnit" :items="COMPUTE_UNIT_ITEMS" size="sm" class="w-full" /></div></div>
        </div>
      </div>

      <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
        <div class="border-b border-default px-4 py-3"><p class="text-sm font-semibold">Deployment</p><p class="mt-0.5 text-xs text-muted">Set traffic and health-check behavior.</p></div>
        <div class="p-4 space-y-3">
          <div class="flex items-center gap-4 px-1"><span class="w-24 text-sm">Port</span><UInput v-model.number="state.port" type="number" :min="1" :max="65535" class="w-32" size="sm" /></div>
          <div class="flex items-center gap-4 px-1"><span class="w-24 text-sm">Replicas</span><UInput v-model.number="state.replicas" type="number" :min="0" class="w-32" size="sm" /></div>
          <div class="flex items-center gap-4 px-1"><span class="w-24 text-sm">Health</span><UInput v-model="state.healthCheckPath" placeholder="/health" class="w-48" size="sm" /></div>
        </div>
      </div>
    </div>

    <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
      <div class="border-b border-default px-4 py-3"><p class="text-sm font-semibold">Region</p><p class="mt-0.5 text-xs text-muted">Choose where this service runs.</p></div>
      <div class="p-4">
        <USelect v-model="regionId" :items="regions.map(r => ({ label: r.display_name, value: r.id }))" placeholder="Select a region" size="sm" class="w-64" />
      </div>
    </div>

    <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
      <div class="border-b border-default px-4 py-3"><p class="text-sm font-semibold">Settings</p><p class="mt-0.5 text-xs text-muted">Control public access to the service.</p></div>
      <div class="p-4"><UCheckbox v-model="state.isPublic" label="Public endpoint" /></div>
    </div>

    <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
      <div class="px-4 py-3 border-b border-default flex items-center justify-between">
        <p class="text-sm font-semibold">Environment Variables</p>
        <UButton variant="solid" size="xs" color="neutral" :icon="ICONS.plus" @click="addEnvRow">Add</UButton>
      </div>
      <div class="p-4 space-y-2">
        <div v-for="(row, i) in envRows" :key="i" class="flex items-center gap-2">
          <UInput v-model="row.key" placeholder="KEY" size="sm" class="w-28" />
          <UDropdownMenu :items="[buildValueMenuItems(i)]" :content="{ align: 'start' }">
            <UButton :label="row.secretId ? `Secret: ${secretNameForId(row.secretId)}` : (row.value || 'Custom value')" trailing-icon="i-lucide-chevrons-up-down" :color="row.secretId ? 'primary' : 'neutral'" variant="soft" size="sm" class="flex-1 justify-between" />
          </UDropdownMenu>
          <UInput v-if="!row.secretId" v-model="row.value" placeholder="value" size="sm" class="w-32" />
          <UButton variant="solid" size="xs" color="error" :icon="ICONS.trash" @click="removeEnvRow(i)" />
        </div>
        <p v-if="envRows.length === 0" class="text-xs text-muted">No environment variables configured.</p>
      </div>
    </div>

    <p v-if="error" class="text-sm text-error">{{ error }}</p>

    <div class="flex justify-end gap-3">
      <UButton variant="ghost" color="neutral" :to="backUrl()">Cancel</UButton>
      <UButton :loading="loading" :disabled="!state.name.trim() || !state.image.trim() || !regionId" @click="handleCreate">Continue</UButton>
    </div>
  </div>
</template>
