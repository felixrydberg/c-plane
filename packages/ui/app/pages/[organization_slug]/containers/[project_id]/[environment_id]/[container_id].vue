<script setup lang="ts">
import type { ContainerVersion } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'
import { loadProjectEnvironments } from '~/utils/auth'

definePageMeta({ key: route => `container-workbench-${route.params.project_id}` })

const store = useStore()
const route = useRoute()
const router = useRouter()
const toast = useToast()

const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() || null)
const environmentId = computed(() => route.params.environment_id?.toString() || null)
const containerId = computed(() => route.params.container_id?.toString() || null)

const projectName = computed(() => store.projects.find(p => p.id === projectId.value)?.name ?? projectId.value ?? '')
const environmentsUrl = computed(() => orgId.value && projectId.value
  ? `/api/cplane/organization/${orgId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/environments` as const
  : '')
const { data: environmentList, refresh: refreshEnvironmentList } = await useFetch(environmentsUrl, {
  immediate: computed(() => !!environmentsUrl.value),
})
const environment = computed(() =>
  environmentList.value?.find(item => item.id === environmentId.value) ?? null
)
const selectedTimelineId = computed(() => {
  const revision = route.query.revision
  return typeof revision === 'string' ? revision : environment.value?.deployed_timeline ?? ''
})
const revisionView = computed<'synced' | 'deployed' | 'draft' | 'historical'>(() => {
  if (
    selectedTimelineId.value === environment.value?.deployed_timeline
    && environment.value?.deployed_timeline === environment.value?.draft_timeline
  ) return 'synced'
  if (selectedTimelineId.value === environment.value?.deployed_timeline) return 'deployed'
  if (selectedTimelineId.value === environment.value?.draft_timeline) return 'draft'
  return 'historical'
})

const activeTab = ref('overview')
const tabs = [
  { label: 'Overview', value: 'overview', slot: 'overview' },
  { label: 'Configuration', value: 'configuration', slot: 'configuration' },
]

  const listUrl = computed(() => orgId.value && projectId.value && environmentId.value
  ? `/api/cplane/organization/${orgId.value as ':organization_id'}/containers` as const
    : '')
const { data: containerList } = await useFetch(listUrl, {
  query: { project_id: projectId, environment_id: environmentId, timeline_id: selectedTimelineId },
  immediate: !!listUrl.value,
})

const name = ref('')
const image = ref('')
const port = ref<number | null>(null)
const replicaCount = ref(1)
const isPublic = ref(false)
const healthCheckPath = ref('')
const envRows = ref<{ key: string; value: string }[]>([])
const hasChanges = ref(false)
const saving = ref(false)
const loading = ref(true)
const loadError = ref('')
const forking = ref(false)
const deployingRevision = ref(false)
const recentActivity = ref<{ refresh: () => Promise<void> } | null>(null)

async function fetchContainer() {
  if (!orgId.value || !containerId.value || !environmentId.value || !selectedTimelineId.value) return
  loading.value = true
  loadError.value = ''
  try {
    const url = `/api/cplane/organization/${orgId.value as ':organization_id'}/containers/${containerId.value as ':container_id'}` as const
    const c = await $fetch(url, { query: { environment_id: environmentId.value, timeline_id: selectedTimelineId.value } })
    name.value = c.name
    if (c.current_version) {
      image.value = c.current_version.image
      port.value = c.current_version.port
      replicaCount.value = c.current_version.replica_count
      isPublic.value = c.current_version.public
      const healthCheck = c.current_version.health_check
      const healthCheckPathValue = healthCheck && typeof healthCheck === 'object' && 'path' in healthCheck ? healthCheck.path : undefined
      healthCheckPath.value = typeof healthCheckPathValue === 'string' ? healthCheckPathValue : ''
      envRows.value = buildEnvRows(c.current_version.env)
    }
    hasChanges.value = false

  } catch {
    loadError.value = 'This container is not present in the selected revision.'
  } finally {
    loading.value = false
  }
}

watch([containerId, selectedTimelineId], fetchContainer, { immediate: true })

function buildEnvRows(env: ContainerVersion['env']): { key: string; value: string }[] {
  if (!env || typeof env !== 'object' || Array.isArray(env)) return []
  return Object.entries(env).map(([key, value]) => ({ key, value: String(value) }))
}

function markChanged() { hasChanges.value = true }

function addEnvRow() { envRows.value.push({ key: '', value: '' }); markChanged() }
function removeEnvRow(i: number) { envRows.value.splice(i, 1); markChanged() }

async function save(autoDeploy: boolean) {
  if (!orgId.value || !containerId.value || !environmentId.value || !selectedTimelineId.value) return
  saving.value = true
  try {
    const env: Record<string, string> = {}
    for (const row of envRows.value) {
      if (!row.key) continue
      env[row.key] = row.value
    }

    await $fetch(`/api/cplane/organization/${orgId.value as ':organization_id'}/containers/${containerId.value as ':container_id'}` as const, {
          method: 'PATCH',
          query: { environment_id: environmentId.value, timeline_id: selectedTimelineId.value },
        body: {
          image: image.value,
          port: port.value,
          replica_count: replicaCount.value,
          public: isPublic.value,
          env: Object.keys(env).length > 0 ? env : null,
          health_check: healthCheckPath.value ? { path: healthCheckPath.value } : null,
          auto_deploy: autoDeploy,
        },
      }
    )
    if (projectId.value && environmentId.value) {
      await loadProjectEnvironments(projectId.value, environmentId.value)
      await refreshEnvironmentList()
    }
    const draftTimelineId = environment.value?.draft_timeline
    await router.replace({
      query: !autoDeploy && draftTimelineId
        ? { ...route.query, revision: draftTimelineId }
        : Object.fromEntries(Object.entries(route.query).filter(([key]) => key !== 'revision')),
    })
    toast.add({ title: autoDeploy ? 'Container updated and deployed' : 'Container draft saved', color: 'success' })
    hasChanges.value = false
    await recentActivity.value?.refresh()
  } catch {
    toast.add({ title: 'Failed to save', color: 'error' })
  } finally {
    saving.value = false
  }
}

async function forkRevision() {
  if (!orgId.value || !projectId.value || !environment.value || !selectedTimelineId.value) return
  forking.value = true
  try {
    await $fetch(
      `/api/cplane/organization/${orgId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/environments/${environment.value.id as ':environment_id'}` as const,
      { method: 'PATCH', body: { draft_timeline_id: selectedTimelineId.value } },
    )
    await loadProjectEnvironments(projectId.value, environmentId.value)
    await refreshEnvironmentList()
    await router.replace({ query: Object.fromEntries(Object.entries(route.query).filter(([key]) => key !== 'revision')) })
    toast.add({ title: 'Revision is now the draft', color: 'success' })
  } catch {
    toast.add({ title: 'Failed to fork revision', color: 'error' })
  } finally {
    forking.value = false
  }
}

async function deployRevision() {
  if (!orgId.value || !projectId.value || !environment.value || !selectedTimelineId.value) return
  deployingRevision.value = true
  try {
    await $fetch(
      `/api/cplane/organization/${orgId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/environments/${environment.value.id as ':environment_id'}` as const,
      { method: 'PATCH', body: { deployed_timeline_id: selectedTimelineId.value } },
    )
    await loadProjectEnvironments(projectId.value, environmentId.value)
    await refreshEnvironmentList()
    toast.add({ title: 'Revision deployed', color: 'success' })
  } catch {
    toast.add({ title: 'Failed to deploy revision', color: 'error' })
  } finally {
    deployingRevision.value = false
  }
}

function backUrl() {
  const orgSlug = route.params.organization_slug?.toString() ?? ''
  const path = `/${orgSlug}/containers/${projectId.value}/${environmentId.value}`
  const revision = route.query.revision
  return typeof revision === 'string' ? `${path}?revision=${encodeURIComponent(revision)}` : path
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

    <div v-else-if="loadError" class="rounded-lg border border-default/60 bg-default p-8 text-center">
      <p class="text-sm font-medium">Container unavailable</p>
      <p class="mt-1 text-sm text-muted">{{ loadError }}</p>
      <UButton class="mt-5" color="neutral" variant="solid" :to="backUrl()">Back to containers</UButton>
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
        <UButton :icon="ICONS.plus" :to="`/${route.params.organization_slug}/containers/${projectId}/${environmentId}/new`">New Container</UButton>
      </header>

      <div class="grid min-h-[720px] xl:grid-cols-[300px_minmax(0,1fr)_280px]">
        <aside class="border-b border-default/60 p-4 xl:border-b-0 xl:border-r">
          <p class="text-sm font-semibold">Containers</p>
          <p class="mt-1 text-xs text-muted">{{ containerList?.length ?? 0 }} in this environment</p>
          <nav class="mt-4 space-y-1" aria-label="Environment containers">
            <NuxtLink
              v-for="container in containerList"
              :key="container.id"
              :to="{ path: `/${route.params.organization_slug}/containers/${projectId}/${environmentId}/${container.id}`, query: { revision: selectedTimelineId } }"
              class="block rounded-md px-3 py-3 transition-colors"
              :class="{'bg-elevated text-highlighted': container.id === containerId}"
            >
              <p class="truncate text-sm font-medium">{{ container.name }}</p>
              <p class="mt-1 truncate font-mono text-[11px] text-muted">{{ container.current_version?.image ?? 'No version' }}</p>
            </NuxtLink>
          </nav>
        </aside>

        <main class="min-w-0 px-5 py-4">
          <UTabs v-model="activeTab" :items="tabs">
            <template #overview>
              <div class="space-y-6 pt-4">
                <div>
                  <h2 class="text-base font-semibold">Overview</h2>
                  <p class="mt-1 text-sm text-muted">Runtime usage for this container.</p>
                </div>

                <div v-if="revisionView === 'draft' || revisionView === 'historical'" class="rounded-md border border-dashed border-default p-5">
                  <p class="text-sm font-medium">This container version is not deployed</p>
                  <p class="mt-1 text-sm text-muted">Metrics are available only for the version running in this environment. Use the project revision switch to view it.</p>
                </div>

                <template v-else>
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
                  <div
                    v-for="stat in [
                    ['Replicas', replicaCount, `Current revision: ${replicaCount}`],
                    ['Restart Count', '—', 'Metrics unavailable'],
                    ['CPU (Current)', '—', 'Metrics unavailable'],
                    ['RAM (Current)', '—', 'Metrics unavailable'],
                  ]" :key="String(stat[0])" class="bg-default px-4 py-3">
                    <dt class="text-xs text-muted">{{ stat[0] }}</dt>
                    <dd class="mt-1 text-lg font-semibold">{{ stat[1] }}</dd>
                    <p class="mt-1 text-[11px] text-muted">{{ stat[2] }}</p>
                  </div>
                </dl>
                </template>
              </div>
            </template>

            <template #configuration>
              <div class="divide-y divide-default/60 pt-4">
                <section class="grid gap-4 py-6 first:pt-2 lg:grid-cols-[180px_minmax(0,1fr)]">
                  <div><h3 class="text-sm font-semibold">Image</h3><p class="mt-1 text-xs text-muted">Container identity and image.</p></div>
                  <div class="space-y-3"><UInput v-model="name" disabled /><UInput v-model="image" placeholder="nginx:latest" :disabled="revisionView !== 'draft' && revisionView !== 'synced'" @input="markChanged" /></div>
                </section>
                <section class="grid gap-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)]">
                  <div><h3 class="text-sm font-semibold">Compute</h3><p class="mt-1 text-xs text-muted">Network port and scale.</p></div>
                  <div class="grid gap-3 sm:grid-cols-2"><UFormField label="Port"><UInput v-model.number="port" type="number" class="w-full" :disabled="revisionView !== 'draft' && revisionView !== 'synced'" @input="markChanged" /></UFormField><UFormField label="Replicas"><UInput v-model.number="replicaCount" type="number" :min="0" class="w-full" :disabled="revisionView !== 'draft' && revisionView !== 'synced'" @input="markChanged" /></UFormField></div>
                </section>
                <section class="grid gap-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)]">
                  <div><h3 class="text-sm font-semibold">Visibility</h3><p class="mt-1 text-xs text-muted">Control service access.</p></div>
                  <UCheckbox v-model="isPublic" label="Publicly accessible" :disabled="revisionView !== 'draft' && revisionView !== 'synced'" @change="markChanged" />
                </section>
                <section class="grid gap-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)]">
                  <div><h3 class="text-sm font-semibold">Health Check</h3><p class="mt-1 text-xs text-muted">Availability endpoint.</p></div>
                  <UInput v-model="healthCheckPath" placeholder="/health" :disabled="revisionView !== 'draft' && revisionView !== 'synced'" @input="markChanged" />
                </section>
                <section class="grid gap-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)]">
                  <div><h3 class="text-sm font-semibold">Environment</h3><p class="mt-1 text-xs text-muted">Custom environment variables.</p></div>
                  <div class="space-y-3">
                    <div v-for="(row, i) in envRows" :key="i" class="grid gap-2 sm:grid-cols-[140px_minmax(0,1fr)_auto]">
                      <UInput v-model="row.key" placeholder="KEY" :disabled="revisionView !== 'draft' && revisionView !== 'synced'" @input="markChanged" />
                      <UInput v-model="row.value" placeholder="value" type="password" :disabled="revisionView !== 'draft' && revisionView !== 'synced'" @input="markChanged" />
                      <UButton size="xs" color="error" :icon="ICONS.trash" :disabled="revisionView !== 'draft' && revisionView !== 'synced'" @click="removeEnvRow(i)">Remove</UButton>
                    </div>
                    <p v-if="envRows.length === 0" class="text-sm text-muted">No environment variables configured.</p>
                    <UButton size="sm" variant="solid" color="neutral" :icon="ICONS.plus" :disabled="revisionView !== 'draft' && revisionView !== 'synced'" @click="addEnvRow">Add Variable</UButton>
                  </div>
                </section>
                <section class="grid gap-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)]">
                  <div><h3 class="text-sm font-semibold">YAML</h3><p class="mt-1 text-xs text-muted">Generated deployment specification.</p></div>
                  <pre class="overflow-x-auto rounded-md bg-elevated/40 p-4 font-mono text-xs text-muted">{{ yamlPreview }}</pre>
                </section>
                <div class="flex justify-end gap-3 py-5">
                  <template v-if="revisionView === 'draft' || revisionView === 'synced'">
                    <UButton variant="ghost" color="neutral" :to="backUrl()">Cancel</UButton>
                    <UButton :icon="ICONS.pencil" color="neutral" variant="solid" :loading="saving" :disabled="!hasChanges" @click="save(false)">Save as draft</UButton>
                    <UButton :icon="ICONS.check" :loading="saving" :disabled="!hasChanges" @click="save(true)">Save &amp; deploy</UButton>
                  </template>
                  <template v-else>
                    <p class="mr-auto text-sm text-muted">Fork this revision to edit it.</p>
                    <UButton :icon="ICONS.pencil" color="neutral" variant="solid" :loading="forking" @click="forkRevision">Fork revision</UButton>
                    <UButton v-if="revisionView === 'historical'" :icon="ICONS.check" :loading="deployingRevision" @click="deployRevision">Deploy revision</UButton>
                  </template>
                </div>
              </div>
            </template>
          </UTabs>
        </main>

        <DeploymentsRecentActivity v-if="orgId && projectId && containerId" ref="recentActivity" :organization-id="orgId" :project-id="projectId" :environment-id="environmentId" event-type-prefix="container" :target-id="containerId" />
      </div>
    </div>
  </div>
</template>
