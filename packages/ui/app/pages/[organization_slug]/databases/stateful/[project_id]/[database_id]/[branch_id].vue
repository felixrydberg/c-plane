<script setup lang="ts">
import { COMPUTE_UNIT_ITEMS, computeUnitByLabel, resolveComputeUnitLabel } from '~/utils/compute-units'
import { ICONS } from '~/utils/icons'

definePageMeta({ key: route => `database-workbench-${route.params.project_id}` })

interface DatabaseBranchRow {
  id: string
  database_id: string
  branch_id: string
  cpu: string | null
  ram: string | null
  high_availability: boolean
  read_replicas: number | null
  autoscaling_enabled: boolean
  autoscaling_min_cpu: string | null
  autoscaling_max_cpu: string | null
}

interface DatabaseRow {
  id: string
  project_id: string
  name: string
  default_branch_id: string | null
}

type BranchItem = { id: string; name: string; timeline: string; is_default: boolean }

const store = useStore()
const route = useRoute()
const toast = useToast()

const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() || null)
const databaseId = computed(() => route.params.database_id?.toString() || null)
const branchId = computed(() => route.params.branch_id?.toString() || null)

const saving = ref(false)
const activeTab = ref('overview')
const tabs = [
  { label: 'Overview', value: 'overview', slot: 'overview' },
  { label: 'Configuration', value: 'configuration', slot: 'configuration' },
  { label: 'Connections', value: 'connections', slot: 'connections' },
  { label: 'Backups', value: 'backups', slot: 'backups' },
]
const cpuPresets = [0.25, 0.5, 1, 2, 4, 8].map(value => ({ label: `${value} cores`, value }))

const computeUnit = ref('0.5')
const state = ref({
  highAvailability: false,
  readReplicas: 1,
  autoscalingEnabled: false,
  autoscalingMinCpu: 0.25,
  autoscalingMaxCpu: 4,
})

const dbName = ref('')
const branchName = ref('')
const isDefault = ref(false)
const defaultDatabaseBranchId = ref<string | null>(null)
const databaseBranches = ref<(DatabaseBranchRow & { _name: string })[]>([])

const loading = ref(true)
const recentActivity = ref<{ refresh: () => Promise<void> } | null>(null)
const currentDatabaseBranch = computed(() => databaseBranches.value.find(branch => branch.branch_id === branchId.value))
const currentDatabaseBranchId = computed(() => currentDatabaseBranch.value?.id ?? '')
const replicas = computed(() => [
  { id: 'primary', name: 'Primary', role: 'Read / Write' },
  ...Array.from({ length: state.value.highAvailability ? Math.max(0, state.value.readReplicas) : 0 }, (_, index) => ({
    id: `replica-${index + 1}`,
    name: `Replica ${index + 1}`,
    role: 'Read only',
  })),
])

async function fetchData() {
  if (!orgId.value || !databaseId.value || !branchId.value || !projectId.value) return
  loading.value = true
  try {
    const [db, branches, projBranches] = await Promise.all([
      $fetch<DatabaseRow>(`/api/backend/organization/${orgId.value}/databases/stateful/${databaseId.value}`),
      $fetch<DatabaseBranchRow[]>(`/api/backend/organization/${orgId.value}/databases/stateful/${databaseId.value}/branches`),
      $fetch<BranchItem[]>(`/api/backend/organization/${orgId.value}/projects/${projectId.value}/branches`),
    ])

    dbName.value = db.name
    defaultDatabaseBranchId.value = db.default_branch_id
    databaseBranches.value = branches.map(branch => ({
      ...branch,
      _name: projBranches.find(projectBranch => projectBranch.id === branch.branch_id)?.name ?? branch.branch_id,
    }))
    const branch = branches.find(b => b.branch_id === branchId.value)
    isDefault.value = db.default_branch_id === branch?.id
    if (branch) {
      computeUnit.value = resolveComputeUnitLabel(branch.cpu, branch.ram)
      state.value = {
        highAvailability: branch.high_availability,
        readReplicas: branch.read_replicas ?? 1,
        autoscalingEnabled: branch.autoscaling_enabled,
        autoscalingMinCpu: branch.autoscaling_min_cpu ? Number.parseFloat(branch.autoscaling_min_cpu) : 0.25,
        autoscalingMaxCpu: branch.autoscaling_max_cpu ? Number.parseFloat(branch.autoscaling_max_cpu) : 4,
      }
    }
    branchName.value = projBranches.find(pb => pb.id === branchId.value)?.name ?? branchId.value
  } catch {
    toast.add({ title: 'Failed to load branch config', color: 'error' })
  } finally {
    loading.value = false
  }
}

watch([databaseId, branchId], fetchData, { immediate: true })

async function save() {
  if (!orgId.value || !databaseId.value || !branchId.value) return
  saving.value = true
  const unit = computeUnitByLabel(computeUnit.value)
  try {
    await $fetch(
      `/api/backend/organization/${orgId.value}/databases/stateful/${databaseId.value}/branches/${branchId.value}`,
      {
        method: 'PATCH',
        body: {
          cpu: `${unit?.cpu ?? 0.5}`,
          ram: `${Math.round((unit?.ramGib ?? 1) * 1024)}Mi`,
          high_availability: state.value.highAvailability,
          read_replicas: state.value.highAvailability ? state.value.readReplicas : null,
          autoscaling_enabled: state.value.autoscalingEnabled,
          autoscaling_min_cpu: `${state.value.autoscalingMinCpu}`,
          autoscaling_max_cpu: `${state.value.autoscalingMaxCpu}`,
        },
      }
    )
    toast.add({ title: 'Branch config saved', color: 'success' })
    await recentActivity.value?.refresh()
  } catch {
    toast.add({ title: 'Failed to save', color: 'error' })
  } finally {
    saving.value = false
  }
}

function backUrl() {
  const orgSlug = route.params.organization_slug?.toString() ?? ''
  return `/${orgSlug}/databases/stateful/${projectId.value}`
}

const connectionString = computed(() => `postgresql://username:password@${dbName.value || 'database'}-${branchName.value || 'branch'}:5432/postgres`)
</script>

<template>
  <div class="w-full max-w-[1500px] mx-auto">
    <div v-if="loading && !dbName" class="flex justify-center py-20"><UIcon name="i-lucide-loader-circle" class="size-5 animate-spin text-muted" /></div>
    <div v-else class="overflow-hidden rounded-lg border border-default/60 bg-default">
      <header class="flex flex-col gap-4 border-b border-default/60 px-5 py-4 sm:flex-row sm:items-center sm:justify-between">
        <div><UiBackLink :label="dbName" :to="backUrl()" /><div class="mt-2 flex items-center gap-2"><h1 class="text-xl font-semibold">{{ dbName }} / {{ branchName }}</h1><UBadge v-if="isDefault" size="sm" variant="soft" color="primary">Default</UBadge></div><p class="mt-1 text-xs text-muted">Stateful Postgres database branch</p></div>
        <UButton :icon="ICONS.plus" :to="`/${route.params.organization_slug}/databases/stateful/${projectId}/new`">New Database</UButton>
      </header>

      <div class="grid min-h-[720px] xl:grid-cols-[270px_minmax(0,1fr)_280px]">
        <aside class="border-b border-default/60 p-4 xl:border-b-0 xl:border-r">
          <p class="text-sm font-semibold">{{ dbName }}</p><p class="mt-1 text-xs text-muted">Database branches</p>
          <nav class="mt-4 space-y-1" aria-label="Database branches">
            <NuxtLink v-for="branch in databaseBranches" :key="branch.id" :to="`/${route.params.organization_slug}/databases/stateful/${projectId}/${databaseId}/${branch.branch_id}`" class="block rounded-md px-3 py-3 transition-colors" :class="{'border-primary bg-elevated': branch.branch_id === branchId}">
              <div class="flex items-center gap-2"><span class="truncate text-sm font-medium">{{ branch._name }}</span><span v-if="branch.id === defaultDatabaseBranchId" class="text-[10px] text-primary">Default</span></div>
              <p class="mt-1 font-mono text-[11px] text-muted">{{ branch.cpu ?? '0.5' }}c &middot; {{ branch.ram ?? '1024Mi' }}</p>
            </NuxtLink>
          </nav>
        </aside>

        <main class="min-w-0 px-5 py-4">
          <Transition mode="out-in" enter-active-class="transition-opacity duration-150 ease-out" enter-from-class="opacity-0" leave-active-class="transition-opacity duration-100 ease-in" leave-to-class="opacity-0">
            <div v-if="loading" key="loading" class="flex min-h-64 items-center justify-center"><UIcon name="i-lucide-loader-circle" class="size-5 animate-spin text-muted" /></div>
            <UTabs v-else key="content" v-model="activeTab" :items="tabs">
            <template #overview>
              <div class="space-y-6 pt-4">
                <div><h2 class="text-base font-semibold">Overview</h2><p class="mt-1 text-sm text-muted">Resource usage for each database replica.</p></div>
                <section v-for="metric in ['CPU Usage', 'RAM Usage']" :key="metric" class="border-b border-default/60 pb-6">
                  <div class="flex items-center justify-between gap-4"><h3 class="text-sm font-semibold">{{ metric }}</h3><span class="font-mono text-xs text-muted">All replicas</span></div>
                  <div class="mt-4 flex min-h-40 flex-col items-center justify-center rounded-md bg-elevated/30 px-6 text-center"><UIcon name="i-heroicons:chart-bar" class="size-6 text-muted" /><p class="mt-3 text-sm font-medium">Telemetry connection pending</p><p class="mt-1 text-sm text-muted">Metrics will appear here once telemetry is available.</p></div>
                </section>
                <section>
                  <div class="flex items-end justify-between gap-4"><div><h3 class="text-sm font-semibold">Replicas</h3><p class="mt-1 text-xs text-muted">CPU and RAM are tracked independently for every instance.</p></div><span class="font-mono text-xs text-muted">{{ replicas.length }} total</span></div>
                  <div class="mt-4 divide-y divide-default/60 rounded-md border border-default/60">
                    <div v-for="replica in replicas" :key="replica.id" class="grid gap-4 px-4 py-4 sm:grid-cols-[minmax(0,1fr)_110px_110px] sm:items-center">
                      <div><p class="text-sm font-medium">{{ replica.name }}</p><p class="mt-1 text-xs text-muted">{{ replica.role }}</p></div>
                      <div><p class="text-[11px] uppercase tracking-wide text-muted">CPU</p><p class="mt-1 font-mono text-sm">&mdash;</p></div>
                      <div><p class="text-[11px] uppercase tracking-wide text-muted">RAM</p><p class="mt-1 font-mono text-sm">&mdash;</p></div>
                    </div>
                  </div>
                </section>
              </div>
            </template>
            <template #configuration>
              <div class="divide-y divide-default/60 pt-4">
                <section class="grid gap-4 py-6 first:pt-2 lg:grid-cols-[180px_minmax(0,1fr)]"><div><h2 class="text-sm font-semibold">Connection String</h2><p class="mt-1 text-xs text-muted">Connect applications to this branch.</p></div><UInput :model-value="connectionString" readonly class="font-mono" /></section>
                <section class="grid gap-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)]"><div><h2 class="text-sm font-semibold">Compute</h2><p class="mt-1 text-xs text-muted">CPU and RAM scale together.</p></div><UFormField label="Compute Unit" description="1 CU includes 1 vCPU and 2 GB RAM."><USelect v-model="computeUnit" :items="COMPUTE_UNIT_ITEMS" class="w-full" /></UFormField></section>
                <section class="grid gap-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)]"><div><h2 class="text-sm font-semibold">High Availability</h2><p class="mt-1 text-xs text-muted">Automatic failover and read replicas.</p></div><div class="space-y-4"><UCheckbox v-model="state.highAvailability" label="Enable high availability" /><UFormField v-if="state.highAvailability" label="Read replicas"><UInput v-model.number="state.readReplicas" type="number" :min="1" class="w-full" /></UFormField></div></section>
                <section class="grid gap-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)]"><div><h2 class="text-sm font-semibold">Autoscaling</h2><p class="mt-1 text-xs text-muted">Grow compute with demand.</p></div><div class="space-y-4"><UCheckbox v-model="state.autoscalingEnabled" label="Enable autoscaling" /><div v-if="state.autoscalingEnabled" class="grid gap-3 sm:grid-cols-2"><UFormField label="Minimum CPU"><USelect v-model="state.autoscalingMinCpu" :items="cpuPresets" class="w-full" /></UFormField><UFormField label="Maximum CPU"><USelect v-model="state.autoscalingMaxCpu" :items="cpuPresets" class="w-full" /></UFormField></div></div></section>
                <div class="flex justify-end gap-3 py-5"><UButton variant="ghost" color="neutral" :to="backUrl()">Cancel</UButton><UButton :icon="ICONS.check" :loading="saving" @click="save">Save Changes</UButton></div>
              </div>
            </template>
            <template #connections><div class="py-10"><h2 class="text-sm font-semibold">Connections</h2><p class="mt-2 text-sm text-muted">Connection pooling and credentials will appear here.</p></div></template>
            <template #backups><div class="py-10"><h2 class="text-sm font-semibold">Backups</h2><p class="mt-2 text-sm text-muted">Backup schedules and restore points will appear here.</p></div></template>
            </UTabs>
          </Transition>
        </main>

        <DeploymentsRecentActivity ref="recentActivity" v-if="orgId && projectId && currentDatabaseBranchId" :organization-id="orgId" :project-id="projectId" :branch-id="branchId" event-type-prefix="database" :target-id="currentDatabaseBranchId" />
      </div>
    </div>
  </div>
</template>
