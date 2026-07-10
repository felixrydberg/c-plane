<script setup lang="ts">
import { ICONS } from '~/utils/icons'

interface ContainerVersionRow {
  id: string
  version: number
  image: string
  public: boolean
  replica_count: number
  port: number | null
  env: Record<string, string> | null
  env_secret_refs: Record<string, string> | null
  health_check: Record<string, unknown> | null
  pull_secret_id: string | null
  created_at: string
}

const store = useStore()
const route = useRoute()
const toast = useToast()

const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() || null)
const branchId = computed(() => route.params.branch_id?.toString() || null)
const containerId = computed(() => route.params.container_id?.toString() || null)

const projectName = computed(() => store.projects.find(p => p.id === projectId.value)?.name ?? projectId.value ?? '')

const name = ref('')
const image = ref('')
const port = ref<number | null>(null)
const replicaCount = ref(1)
const isPublic = ref(false)
const healthCheckPath = ref('')
const envRows = ref<{ key: string; value: string; secretId: string | null }[]>([])
const hasChanges = ref(false)
const saving = ref(false)
const loading = ref(true)

interface ProjectSecret {
  id: string
  name: string
  version_count: number
}
const projectSecrets = ref<ProjectSecret[]>([])

async function fetchContainer() {
  if (!orgId.value || !containerId.value) return
  loading.value = true
  try {
    const c = await $fetch<{ name: string; current_version: ContainerVersionRow | null }>(
      `/api/backend/organization/${orgId.value}/containers/${containerId.value}`
    )
    name.value = c.name
    if (c.current_version) {
      image.value = c.current_version.image
      port.value = c.current_version.port
      replicaCount.value = c.current_version.replica_count
      isPublic.value = c.current_version.public
      const healthCheckPath = c.current_version.health_check?.path
      healthCheckPath.value = typeof healthCheckPath === 'string' ? healthCheckPath : ''
      envRows.value = buildEnvRows(c.current_version.env, c.current_version.env_secret_refs)
    }
    hasChanges.value = false

    const res = await $fetch<{ data?: ProjectSecret[] }>(
      `/api/backend/organization/${orgId.value}/projects/${projectId.value}/secrets`
    )
    projectSecrets.value = (res?.data ?? res ?? []) as ProjectSecret[]
  } catch {
    toast.add({ title: 'Failed to load container', color: 'error' })
  } finally {
    loading.value = false
  }
}

onMounted(() => { fetchContainer() })

function buildEnvRows(env: Record<string, string> | null, refs: Record<string, string> | null): { key: string; value: string; secretId: string | null }[] {
  const rows: { key: string; value: string; secretId: string | null }[] = []
  if (env) {
    for (const [key, value] of Object.entries(env)) {
      rows.push({ key, value: refs?.[key] ? '' : value, secretId: refs?.[key] ?? null })
    }
  }
  if (refs) {
    for (const [key, secretId] of Object.entries(refs)) {
      if (!rows.some(r => r.key === key)) {
        rows.push({ key, value: '', secretId: secretId ?? null })
      }
    }
  }
  return rows
}

function markChanged() { hasChanges.value = true }

function addEnvRow() { envRows.value.push({ key: '', value: '', secretId: null }); markChanged() }
function removeEnvRow(i: number) { envRows.value.splice(i, 1); markChanged() }
function setRowSecret(i: number, secretId: string) { envRows.value[i].secretId = secretId; envRows.value[i].value = ''; markChanged() }

async function save() {
  if (!orgId.value || !containerId.value) return
  saving.value = true
  try {
    const env: Record<string, string> = {}
    const env_secret_refs: Record<string, string> = {}
    for (const row of envRows.value) {
      if (!row.key) continue
      if (row.secretId) {
        env_secret_refs[row.key] = row.secretId
      } else {
        env[row.key] = row.value
      }
    }

    await $fetch(
      `/api/backend/organization/${orgId.value}/containers/${containerId.value}?branch_id=${branchId.value ?? ''}`,
      {
        method: 'PATCH',
        body: {
          image: image.value,
          port: port.value,
          replica_count: replicaCount.value,
          public: isPublic.value,
          env: Object.keys(env).length > 0 ? env : null,
          env_secret_refs: Object.keys(env_secret_refs).length > 0 ? env_secret_refs : null,
          health_check: healthCheckPath.value ? { path: healthCheckPath.value } : null,
        },
      }
    )
    toast.add({ title: 'Container updated', color: 'success' })
    hasChanges.value = false
  } catch {
    toast.add({ title: 'Failed to save', color: 'error' })
  } finally {
    saving.value = false
  }
}

function backUrl() {
  const orgSlug = route.params.organization_slug?.toString() ?? ''
  return `/${orgSlug}/containers/${projectId.value}/${branchId.value}`
}

watch([image, port, replicaCount, isPublic, healthCheckPath], () => markChanged())
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div>
      <UiBackLink :label="projectName" :to="backUrl()" />
      <div class="mt-2">
        <h1 class="text-2xl font-semibold">{{ name }}</h1>
        <p class="mt-0.5 text-sm text-muted">Container configuration</p>
      </div>
    </div>

    <div v-if="loading" class="text-center py-8">
      <UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" />
    </div>

    <template v-else>
      <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
        <div class="px-4 py-3 border-b border-default">
          <p class="text-sm font-semibold">Image</p>
          <p class="mt-0.5 text-xs text-muted">The image to run for this service.</p>
        </div>
        <div class="p-4 space-y-3">
          <div class="flex items-center gap-4 px-1">
            <span class="w-24 text-sm">Name</span>
            <UInput v-model="name" class="flex-1" size="sm" disabled />
          </div>
          <div class="flex items-center gap-4 px-1">
            <span class="w-24 text-sm">Image</span>
            <UInput v-model="image" placeholder="e.g. nginx:latest" class="flex-1" size="sm" @input="markChanged" />
          </div>
        </div>
      </div>

      <div class="grid gap-6 lg:grid-cols-2">
      <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
        <div class="px-4 py-3 border-b border-default">
          <p class="text-sm font-semibold">Compute</p>
          <p class="mt-0.5 text-xs text-muted">Network port and desired scale.</p>
        </div>
        <div class="p-4 space-y-3">
          <div class="flex items-center gap-4 px-1">
            <span class="w-24 text-sm">Port</span>
            <UInput v-model.number="port" type="number" placeholder="80" class="w-32" size="sm" @input="markChanged" />
          </div>
          <div class="flex items-center gap-4 px-1">
            <span class="w-24 text-sm">Replicas</span>
            <UInput v-model.number="replicaCount" type="number" :min="0" class="w-32" size="sm" @input="markChanged" />
          </div>
        </div>
      </div>

      <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
        <div class="px-4 py-3 border-b border-default">
          <p class="text-sm font-semibold">Visibility</p>
          <p class="mt-0.5 text-xs text-muted">Control how this service is reachable.</p>
        </div>
        <div class="p-4">
          <UCheckbox v-model="isPublic" label="Publicly accessible" @change="markChanged" />
        </div>
      </div>

      </div>

      <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
        <div class="px-4 py-3 border-b border-default">
          <p class="text-sm font-semibold">Health Check</p>
          <p class="mt-0.5 text-xs text-muted">Optional endpoint used to check availability.</p>
        </div>
        <div class="p-4">
          <div class="flex items-center gap-4 px-1">
            <span class="w-24 text-sm">Path</span>
            <UInput v-model="healthCheckPath" placeholder="/health" class="w-48" size="sm" @input="markChanged" />
          </div>
        </div>
      </div>

      <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
        <div class="px-4 py-3 border-b border-default flex items-center justify-between">
          <p class="text-sm font-semibold">Environment</p>
          <UButton variant="solid" size="xs" color="neutral" :icon="ICONS.plus" @click="addEnvRow">Add</UButton>
        </div>
        <div class="p-4 space-y-2">
          <div v-if="envRows.length === 0" class="text-sm text-muted py-2 text-center">
            No environment variables configured.
          </div>
          <div v-for="(row, i) in envRows" :key="i" class="flex items-center gap-2">
            <UInput v-model="row.key" placeholder="KEY" class="w-40" size="sm" @input="markChanged" />
            <span class="text-muted text-xs">=</span>
            <UInput v-if="!row.secretId" v-model="row.value" placeholder="value" class="flex-1" size="sm" type="password" @input="markChanged" />
            <USelect
              v-model="row.secretId"
              :items="[{ label: 'Use secret...', value: '' }, ...projectSecrets.map(s => ({ label: s.name, value: s.id }))]"
              placeholder="Use secret..."
              size="sm"
              class="w-44"
              @update:model-value="(v: string) => setRowSecret(i, v)"
            />
            <UButton variant="solid" size="xs" color="error" :icon="ICONS.trash" @click="removeEnvRow(i)" />
          </div>
        </div>
      </div>

      <div class="flex justify-end gap-3">
        <UButton variant="ghost" color="neutral" :to="backUrl()">Cancel</UButton>
        <UButton :loading="saving" :disabled="!hasChanges" @click="save">Save</UButton>
      </div>
    </template>
  </div>
</template>
