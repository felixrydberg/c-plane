<script setup lang="ts">
import type { ContainerVersion } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'
import { loadProjectEnvironments } from '~/utils/auth'
import { getErrorMessage } from '~/utils/errors'

definePageMeta({ key: route => `container-workbench-${route.params.project_id}` })

const store = useStore()
const route = useRoute()
const router = useRouter()
const toast = useToast()

const organizationSlug = computed(() => route.params.organization_slug?.toString() || '')
const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() || null)
const environmentId = computed(() => route.params.environment_id?.toString() || null)
const containerId = computed(() => route.params.container_id?.toString() || null)
const externalRegistriesUrl = computed(() => orgId.value
  ? `/api/organization/${orgId.value as ':organization_id'}/registry/external-registries` as const
  : '')
const { data: externalRegistries } = await useCplaneFetch(externalRegistriesUrl, { default: () => [] })
const externalRegistryItems = computed(() => [
  { label: 'No external registry', value: 'none' },
  ...externalRegistries.value.map(registry => ({
    label: `${registry.name} — ${registry.host} (${registry.username})`,
    value: registry.id,
  })),
])

const projectName = computed(() => store.projects.find(p => p.id === projectId.value)?.name ?? projectId.value ?? '')
const environmentsUrl = computed(() => orgId.value && projectId.value
  ? `/api/organization/${orgId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/environments` as const
  : '')
const { data: environmentList, refresh: refreshEnvironmentList } = await useCplaneFetch(environmentsUrl, {
  immediate: computed(() => !!environmentsUrl.value),
})
const environment = computed(() =>
  environmentList.value?.find(item => item.id === environmentId.value) ?? null
)
const selectedTimelineId = computed(() => {
  const revision = route.query.revision
  return typeof revision === 'string' ? revision : environment.value?.deployed_timeline
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
  { label: 'Usage', value: 'overview', slot: 'overview' },
  { label: 'Configuration', value: 'configuration', slot: 'configuration' },
]

const name = ref('')
const image = ref('')
const externalRegistryId = ref('none')
const port = ref<number | null>(null)
const replicaCount = ref(1)
const isPublic = ref(false)
const healthCheckPath = ref('')
const envRows = ref<{ key: string; value: string }[]>([])
const hasChanges = ref(false)
const saving = ref(false)
const refreshing = ref(false)
const loading = ref(true)
const loadError = ref('')
const forking = ref(false)
const deployingRevision = ref(false)
const recentActivity = ref<{ refresh: () => Promise<void> } | null>(null)

watch(name, () => {
  if (!name.value || !containerId.value || !organizationSlug.value) return
  store.setBreadcrumbs([
    { label: 'Containers', to: backUrl() },
    { label: name.value },
  ])
}, { immediate: true })

async function fetchContainer() {
  if (!orgId.value || !containerId.value || !environmentId.value || !selectedTimelineId.value) {
    loading.value = false
    return
  }
  loading.value = true
  loadError.value = ''
  try {
    const url = `/api/organization/${orgId.value as ':organization_id'}/containers/${containerId.value as ':container_id'}` as const
    const c = await cplaneFetch(url, { query: { environment_id: environmentId.value, timeline_id: selectedTimelineId.value } })
    name.value = c.name
    if (c.current_version) {
      image.value = c.current_version.image
      externalRegistryId.value = c.current_version.external_registry_id ?? 'none'
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

    await cplaneFetch(`/api/organization/${orgId.value as ':organization_id'}/containers/${containerId.value as ':container_id'}` as const, {
          method: 'PATCH',
          query: { environment_id: environmentId.value, timeline_id: selectedTimelineId.value },
        body: {
          image: image.value,
          external_registry_id: externalRegistryId.value === 'none' ? null : externalRegistryId.value,
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
  } catch (e: unknown) {
    const message = getErrorMessage(e, '')
    toast.add({ title: 'Failed to save', description: message, color: 'error' })
  } finally {
    saving.value = false
  }
}

async function forkRevision() {
  if (!orgId.value || !projectId.value || !environment.value || !selectedTimelineId.value) return
  forking.value = true
  try {
    await cplaneFetch(
      `/api/organization/${orgId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/environments/${environment.value.id as ':environment_id'}` as const,
      { method: 'PATCH', body: { draft_timeline_id: selectedTimelineId.value } },
    )
    await loadProjectEnvironments(projectId.value, environmentId.value)
    await refreshEnvironmentList()
    await router.replace({ query: { ...route.query, revision: selectedTimelineId.value } })
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
    await cplaneFetch(
      `/api/organization/${orgId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/environments/${environment.value.id as ':environment_id'}` as const,
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
  `externalRegistry: ${externalRegistryId.value === 'none' ? 'null' : externalRegistryId.value}`,
  `port: ${port.value ?? 'null'}`,
  `replicas: ${replicaCount.value}`,
  `public: ${isPublic.value}`,
  'healthCheck:',
  `  path: ${healthCheckPath.value || 'null'}`,
].join('\n'))
const canRefreshLatest = computed(() => image.value.trim().endsWith(':latest'))

async function refreshLatest() {
  if (!orgId.value || !containerId.value || !environmentId.value || !selectedTimelineId.value || hasChanges.value) return
  refreshing.value = true
  try {
    await cplaneFetch(`/api/organization/${orgId.value as ':organization_id'}/containers/${containerId.value as ':container_id'}/deploy` as const, {
      query: { environment_id: environmentId.value, timeline_id: selectedTimelineId.value },
      method: 'POST',
    })
    if (projectId.value && environmentId.value) {
      await loadProjectEnvironments(projectId.value, environmentId.value)
      await refreshEnvironmentList()
    }
    await router.replace({
      query: Object.fromEntries(Object.entries(route.query).filter(([key]) => key !== 'revision')),
    })
    toast.add({ title: 'Latest image refreshed and deployed', color: 'success' })
    await recentActivity.value?.refresh()
  } catch (e: unknown) {
    toast.add({ title: 'Failed to refresh latest image', description: getErrorMessage(e, ''), color: 'error' })
  } finally {
    refreshing.value = false
  }
}

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
      <UButton class="mt-5" color="neutral" :to="backUrl()">Back to Containers</UButton>
    </div>

    <div v-else class="overflow-hidden rounded-lg border border-default/60 bg-default">
      <header class="flex flex-col gap-4 border-b border-default/60 px-5 py-4 sm:flex-row sm:items-center sm:justify-between">
        <div class="min-w-0">
          <UiBackLink :label="projectName" :to="backUrl()" />
          <UiPageEyebrow label="Compute" />
          <div class="mt-2 flex flex-wrap items-center gap-2">
            <h1 class="truncate text-xl font-semibold">{{ name }}</h1>
            <UBadge :color="revisionView === 'draft' ? 'warning' : 'success'" variant="soft" size="sm">
              {{ revisionView === 'draft' ? 'Draft' : revisionView === 'historical' ? 'Historical' : 'Deployed' }}
            </UBadge>
          </div>
          <p class="mt-1 text-xs text-muted">{{ environment?.name ?? environmentId }} · {{ image }} · Port {{ port ?? 'none' }} · {{ replicaCount }} replica{{ replicaCount === 1 ? '' : 's' }}</p>
        </div>
      </header>

      <div class="grid min-h-180 xl:grid-cols-[minmax(0,1fr)_280px]">
        <main class="min-w-0 px-5 py-4">
          <UiTabs v-model="activeTab" :items="tabs">
            <template #overview>
              <div class="space-y-6 pt-4">
                <div>
                  <div class="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
                    <div>
                      <h2 class="text-base font-semibold">Usage</h2>
                      <p class="mt-1 text-sm text-muted">Runtime usage for this container.</p>
                    </div>
                    <UButton :icon="ICONS.calendar" color="neutral" variant="outline">Last 24 hours</UButton>
                  </div>
                </div>

                <div v-if="revisionView === 'draft' || revisionView === 'historical'" class="rounded-md border border-dashed border-default p-5">
                  <p class="text-sm font-medium">This container version is not deployed</p>
                  <p class="mt-1 text-sm text-muted">Metrics are available only for the version running in this environment. Use the project revision switch to view it.</p>
                </div>

                <template v-else>
                <dl class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
                  <div v-for="stat in [
                    ['Replicas', replicaCount, 'Current revision'],
                    ['Restart Count', '—', 'Metrics unavailable'],
                    ['CPU (Current)', '—', 'Metrics unavailable'],
                    ['RAM (Current)', '—', 'Metrics unavailable'],
                  ]" :key="String(stat[0])" class="rounded-lg border border-default/60 bg-default px-4 py-3">
                    <dt class="text-xs text-muted">{{ stat[0] }}</dt>
                    <dd class="mt-1 text-xl font-semibold">{{ stat[1] }}</dd>
                    <p class="mt-1 text-[11px] text-muted">{{ stat[2] }}</p>
                  </div>
                </dl>

                <section v-for="metric in ['CPU Usage', 'RAM Usage']" :key="metric" class="overflow-hidden rounded-lg border border-default/60 bg-default">
                  <div class="border-b border-default/60 bg-elevated/20 px-4 py-2.5">
                    <div class="flex items-center justify-between gap-4">
                      <h3 class="text-sm font-medium text-muted">{{ metric }}</h3>
                      <span class="font-mono text-xs text-muted">Usage · Request · Limit</span>
                    </div>
                  </div>
                  <div class="p-4">
                    <div class="flex min-h-40 items-center justify-center rounded-md border border-dashed border-default/60 px-6 text-center">
                      <div>
                        <UIcon name="i-heroicons:chart-bar" class="size-6 text-muted" />
                        <p class="mt-3 text-sm font-medium">Telemetry connection pending</p>
                        <p class="mt-1 text-sm text-muted">Metrics will appear here once telemetry is available.</p>
                      </div>
                    </div>
                  </div>
                </section>
                </template>
              </div>
            </template>

            <template #configuration>
              <div class="divide-y divide-default/60 pt-4">
                <section class="grid gap-4 py-6 first:pt-2 lg:grid-cols-[180px_minmax(0,1fr)]">
                  <div><h3 class="text-sm font-semibold">Image</h3><p class="mt-1 text-xs text-muted">Container identity and image.</p></div>
                  <div class="space-y-3">
                    <UInput v-model="name" disabled class="w-full" />
                    <div class="flex gap-2">
                      <UInput v-model="image" placeholder="nginx:latest" class="min-w-0 flex-1" :disabled="revisionView !== 'draft' && revisionView !== 'synced'" @input="markChanged" />
                      <UButton v-if="canRefreshLatest && (revisionView === 'draft' || revisionView === 'synced')" :icon="ICONS.refresh" color="neutral" :loading="refreshing" :disabled="hasChanges" @click="refreshLatest">Refresh latest</UButton>
                    </div>
                    <UFormField label="External registry" description="Optional credentials for a private image.">
                      <USelect v-model="externalRegistryId" :items="externalRegistryItems" class="w-full" :disabled="revisionView !== 'draft' && revisionView !== 'synced'" @change="markChanged" />
                    </UFormField>
                  </div>
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
                    <UButton size="sm" color="neutral" :icon="ICONS.plus" :disabled="revisionView !== 'draft' && revisionView !== 'synced'" @click="addEnvRow">Add Variable</UButton>
                  </div>
                </section>
                <section class="grid gap-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)]">
                  <div><h3 class="text-sm font-semibold">YAML</h3><p class="mt-1 text-xs text-muted">Generated deployment specification.</p></div>
                  <pre class="overflow-x-auto rounded-md bg-elevated/40 p-4 font-mono text-xs text-muted">{{ yamlPreview }}</pre>
                </section>
                <div class="flex justify-end gap-3 py-5">
                  <template v-if="revisionView === 'draft' || revisionView === 'synced'">
                    <UButton variant="ghost" color="neutral" :to="backUrl()">Cancel</UButton>
                    <UButton :icon="ICONS.pencil" color="neutral" :loading="saving" :disabled="!hasChanges" @click="save(false)">Save as draft</UButton>
                    <UButton :icon="ICONS.check" :loading="saving" :disabled="!hasChanges" @click="save(true)">Save &amp; deploy</UButton>
                  </template>
                  <template v-else>
                    <p class="mr-auto text-sm text-muted">Fork this revision to edit it.</p>
                    <UButton :icon="ICONS.pencil" color="neutral" :loading="forking" @click="forkRevision">Fork revision</UButton>
                    <UButton v-if="revisionView === 'historical'" :icon="ICONS.check" :loading="deployingRevision" @click="deployRevision">Deploy revision</UButton>
                  </template>
                </div>
              </div>
            </template>
          </UiTabs>
        </main>

        <DeploymentsRecentActivity v-if="orgId && projectId && containerId" ref="recentActivity" :organization-id="orgId" :project-id="projectId" :environment-id="environmentId" event-type-prefix="container" :target-id="containerId" />
      </div>
    </div>
  </div>
</template>
