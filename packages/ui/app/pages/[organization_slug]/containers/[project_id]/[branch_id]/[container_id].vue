<script setup lang="ts">
import { ICONS } from '~/utils/icons'

definePageMeta({ key: route => `container-workbench-${route.params.project_id}` })

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

const activeTab = ref('overview')
const tabs = [
  { label: 'Overview', value: 'overview', slot: 'overview' },
  { label: 'Configuration', value: 'configuration', slot: 'configuration' },
]

interface ContainerListItem {
  id: string
  name: string
  current_version: ContainerVersionRow | null
}

const listUrl = computed(() => orgId.value && projectId.value && branchId.value
  ? `/api/backend/organization/${orgId.value}/containers`
  : '')
const { data: containerList } = await useFetch<ContainerListItem[]>(listUrl, {
  query: { project_id: projectId, branch_id: branchId },
  immediate: !!listUrl.value,
})

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
const recentActivity = ref<{ refresh: () => Promise<void> } | null>(null)

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

watch(containerId, fetchContainer, { immediate: true })

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
    await recentActivity.value?.refresh()
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

const yamlPreview = computed(() => [
  `name: ${name.value}`,
  `image: ${image.value}`,
  `port: ${port.value ?? 'null'}`,
  `replicas: ${replicaCount.value}`,
  `public: ${isPublic.value}`,
  'healthCheck:',
  `  path: ${healthCheckPath.value || 'null'}`,
].join('\n'))

watch([image, port, replicaCount, isPublic, healthCheckPath], () => markChanged())
</script>

<template>
  <div class="w-full max-w-[1500px] mx-auto">
    <div v-if="loading && !name" class="flex justify-center py-20">
      <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin text-muted" />
    </div>

    <div v-else class="overflow-hidden rounded-lg border border-default/60 bg-default">
      <header class="flex flex-col gap-4 border-b border-default/60 px-5 py-4 sm:flex-row sm:items-center sm:justify-between">
        <div class="min-w-0">
          <UiBackLink :label="projectName" :to="backUrl()" />
          <div class="mt-2 flex flex-wrap items-center gap-2">
            <h1 class="truncate text-xl font-semibold">{{ name }}</h1>
            <span class="font-mono text-xs text-muted">{{ image }}</span>
            <span class="text-xs text-muted">&middot; Port {{ port ?? 'none' }}</span>
            <span class="text-xs text-muted">&middot; {{ replicaCount }} replica{{ replicaCount === 1 ? '' : 's' }}</span>
          </div>
        </div>
        <UButton :icon="ICONS.plus" :to="`/${route.params.organization_slug}/containers/${projectId}/${branchId}/new`">New Container</UButton>
      </header>

      <div class="grid min-h-[720px] xl:grid-cols-[300px_minmax(0,1fr)_280px]">
        <aside class="border-b border-default/60 p-4 xl:border-b-0 xl:border-r">
          <p class="text-sm font-semibold">Containers</p>
          <p class="mt-1 text-xs text-muted">{{ containerList?.length ?? 0 }} in this branch</p>
          <nav class="mt-4 space-y-1" aria-label="Branch containers">
            <NuxtLink
              v-for="container in containerList"
              :key="container.id"
              :to="`/${route.params.organization_slug}/containers/${projectId}/${branchId}/${container.id}`"
              class="block rounded-md px-3 py-3 transition-colors"
              :class="{'bg-elevated text-highlighted': container.id === containerId}"
            >
              <p class="truncate text-sm font-medium">{{ container.name }}</p>
              <p class="mt-1 truncate font-mono text-[11px] text-muted">{{ container.current_version?.image ?? 'No version' }}</p>
            </NuxtLink>
          </nav>
        </aside>

        <main class="min-w-0 px-5 py-4">
          <Transition mode="out-in" enter-active-class="transition-opacity duration-150 ease-out" enter-from-class="opacity-0" leave-active-class="transition-opacity duration-100 ease-in" leave-to-class="opacity-0">
            <div v-if="loading" key="loading" class="flex min-h-64 items-center justify-center"><UIcon name="i-lucide-loader-circle" class="size-5 animate-spin text-muted" /></div>
            <UTabs v-else key="content" v-model="activeTab" :items="tabs">
            <template #overview>
              <div class="space-y-6 pt-4">
                <div>
                  <h2 class="text-base font-semibold">Overview</h2>
                  <p class="mt-1 text-sm text-muted">Runtime usage for this container.</p>
                </div>

                <section v-for="metric in ['CPU Usage', 'RAM Usage']" :key="metric" class="border-b border-default/60 pb-6">
                  <div class="flex items-center justify-between gap-4">
                    <h3 class="text-sm font-semibold">{{ metric }}</h3>
                    <span class="font-mono text-xs text-muted">Usage &middot; Request &middot; Limit</span>
                  </div>
                  <div class="mt-4 flex min-h-48 flex-col items-center justify-center rounded-md bg-elevated/30 px-6 text-center">
                    <UIcon name="i-heroicons:chart-bar" class="size-6 text-muted" />
                    <p class="mt-3 text-sm font-medium">Telemetry connection pending</p>
                    <p class="mt-1 text-sm text-muted">Metrics will appear here once telemetry is available.</p>
                  </div>
                </section>

                <dl class="grid gap-px overflow-hidden rounded-md bg-default/60 sm:grid-cols-2 lg:grid-cols-4">
                  <div v-for="stat in [
                    ['Replicas', replicaCount, `Desired: ${replicaCount}`],
                    ['Restart Count', '—', 'Metrics unavailable'],
                    ['CPU (Current)', '—', 'Metrics unavailable'],
                    ['RAM (Current)', '—', 'Metrics unavailable'],
                  ]" :key="String(stat[0])" class="bg-default px-4 py-3">
                    <dt class="text-xs text-muted">{{ stat[0] }}</dt>
                    <dd class="mt-1 text-lg font-semibold">{{ stat[1] }}</dd>
                    <p class="mt-1 text-[11px] text-muted">{{ stat[2] }}</p>
                  </div>
                </dl>
              </div>
            </template>

            <template #configuration>
              <div class="divide-y divide-default/60 pt-4">
                <section class="grid gap-4 py-6 first:pt-2 lg:grid-cols-[180px_minmax(0,1fr)]">
                  <div><h3 class="text-sm font-semibold">Image</h3><p class="mt-1 text-xs text-muted">Container identity and image.</p></div>
                  <div class="space-y-3"><UInput v-model="name" disabled /><UInput v-model="image" placeholder="nginx:latest" @input="markChanged" /></div>
                </section>
                <section class="grid gap-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)]">
                  <div><h3 class="text-sm font-semibold">Compute</h3><p class="mt-1 text-xs text-muted">Network port and scale.</p></div>
                  <div class="grid gap-3 sm:grid-cols-2"><UFormField label="Port"><UInput v-model.number="port" type="number" class="w-full" @input="markChanged" /></UFormField><UFormField label="Replicas"><UInput v-model.number="replicaCount" type="number" :min="0" class="w-full" @input="markChanged" /></UFormField></div>
                </section>
                <section class="grid gap-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)]">
                  <div><h3 class="text-sm font-semibold">Visibility</h3><p class="mt-1 text-xs text-muted">Control service access.</p></div>
                  <UCheckbox v-model="isPublic" label="Publicly accessible" @change="markChanged" />
                </section>
                <section class="grid gap-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)]">
                  <div><h3 class="text-sm font-semibold">Health Check</h3><p class="mt-1 text-xs text-muted">Availability endpoint.</p></div>
                  <UInput v-model="healthCheckPath" placeholder="/health" @input="markChanged" />
                </section>
                <section class="grid gap-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)]">
                  <div><h3 class="text-sm font-semibold">Environment</h3><p class="mt-1 text-xs text-muted">Values and project secrets.</p></div>
                  <div class="space-y-3">
                    <div v-for="(row, i) in envRows" :key="i" class="grid gap-2 sm:grid-cols-[140px_minmax(0,1fr)_170px_auto]">
                      <UInput v-model="row.key" placeholder="KEY" @input="markChanged" />
                      <UInput v-if="!row.secretId" v-model="row.value" placeholder="value" type="password" @input="markChanged" />
                      <div v-else class="rounded-md bg-elevated px-3 py-2 text-sm text-muted">Project secret</div>
                      <USelect v-model="row.secretId" :items="[{ label: 'Custom value', value: '' }, ...projectSecrets.map(s => ({ label: s.name, value: s.id }))]" @update:model-value="(v: string) => setRowSecret(i, v)" />
                      <UButton size="xs" color="error" :icon="ICONS.trash" @click="removeEnvRow(i)">Remove</UButton>
                    </div>
                    <p v-if="envRows.length === 0" class="text-sm text-muted">No environment variables configured.</p>
                    <UButton size="sm" variant="solid" color="neutral" :icon="ICONS.plus" @click="addEnvRow">Add Variable</UButton>
                  </div>
                </section>
                <section class="grid gap-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)]">
                  <div><h3 class="text-sm font-semibold">YAML</h3><p class="mt-1 text-xs text-muted">Generated deployment specification.</p></div>
                  <pre class="overflow-x-auto rounded-md bg-elevated/40 p-4 font-mono text-xs text-muted">{{ yamlPreview }}</pre>
                </section>
                <div class="flex justify-end gap-3 py-5">
                  <UButton variant="ghost" color="neutral" :to="backUrl()">Cancel</UButton>
                  <UButton :icon="ICONS.check" :loading="saving" :disabled="!hasChanges" @click="save">Save Changes</UButton>
                </div>
              </div>
            </template>
            </UTabs>
          </Transition>
        </main>

        <DeploymentsRecentActivity ref="recentActivity" v-if="orgId && projectId && containerId" :organization-id="orgId" :project-id="projectId" :branch-id="branchId" event-type-prefix="container" :target-id="containerId" />
      </div>
    </div>
  </div>
</template>
