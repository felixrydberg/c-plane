<script setup lang="ts">
import { COMPUTE_UNIT_ITEMS, computeUnitByLabel, resolveComputeUnitLabel } from '~/utils/compute-units'

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

const computeUnit = ref('XS')
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

const loading = ref(true)

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

onMounted(() => { fetchData() })

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
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div>
      <UiBackLink :label="dbName" :to="backUrl()" />
      <div class="mt-2 min-w-0">
        <div class="flex items-center gap-2">
          <h1 class="truncate text-2xl font-semibold">{{ branchName }}</h1>
          <UBadge v-if="isDefault" size="sm" variant="soft" color="info">default</UBadge>
        </div>
        <p class="mt-0.5 text-sm text-muted">{{ dbName }} database branch</p>
      </div>
    </div>

    <div v-if="loading" class="text-center py-8">
      <UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" />
    </div>

    <template v-else>
      <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
        <div class="px-4 py-3 border-b border-default">
          <p class="text-sm font-semibold">Connection string</p>
          <p class="mt-0.5 text-xs text-muted">Use this to connect applications to this database branch.</p>
        </div>
        <div class="p-4">
          <UInput placeholder="postgresql://username:password@host:5432/database" readonly class="w-full font-mono" size="sm" />
        </div>
      </div>

      <div class="grid gap-6 lg:grid-cols-2">
        <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
        <div class="px-4 py-3 border-b border-default">
          <p class="text-sm font-semibold">Compute</p>
          <p class="mt-0.5 text-xs text-muted">CPU and RAM scale together as a compute unit.</p>
        </div>
        <div class="p-4">
          <div class="flex items-center gap-4 px-1">
            <span class="w-28 text-sm">Compute Unit</span>
            <div class="w-28">
              <USelect v-model="computeUnit" :items="COMPUTE_UNIT_ITEMS" size="sm" class="w-full" />
            </div>
          </div>
        </div>
      </div>

        <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
        <div class="px-4 py-3 border-b border-default">
          <p class="text-sm font-semibold">High Availability</p>
          <p class="mt-0.5 text-xs text-muted">Add replicas for increased resilience.</p>
        </div>
        <div class="p-4 space-y-3">
          <UCheckbox v-model="state.highAvailability" label="Enable high availability" />
          <template v-if="state.highAvailability">
            <div class="flex items-center gap-4 px-1">
              <span class="w-24 text-sm">Replicas</span>
              <div class="w-28">
                <UInput v-model.number="state.readReplicas" type="number" :min="1" size="sm" class="w-full" />
              </div>
              <span class="text-xs text-muted">read replicas</span>
            </div>
          </template>
        </div>
        </div>

      </div>

      <div class="w-full overflow-hidden rounded-lg border border-default bg-default">
        <div class="px-4 py-3 border-b border-default">
          <p class="text-sm font-semibold">Autoscaling</p>
          <p class="mt-0.5 text-xs text-muted">Let CPU capacity grow with demand.</p>
        </div>
        <div class="p-4 space-y-3">
          <UCheckbox v-model="state.autoscalingEnabled" label="Enable autoscaling" />
          <template v-if="state.autoscalingEnabled">
            <div class="flex items-center gap-4 px-1">
              <span class="w-24 text-sm">Min CPU</span>
              <div class="w-28">
                <USelect v-model="state.autoscalingMinCpu" :items="cpuPresets" size="sm" class="w-full" />
              </div>
              <span class="text-xs text-muted">cores</span>
            </div>
            <div class="flex items-center gap-4 px-1">
              <span class="w-24 text-sm">Max CPU</span>
              <div class="w-28">
                <USelect v-model="state.autoscalingMaxCpu" :items="cpuPresets" size="sm" class="w-full" />
              </div>
              <span class="text-xs text-muted">cores</span>
            </div>
          </template>
        </div>
      </div>

      <div class="flex items-center justify-between rounded-lg border border-default bg-elevated/30 px-4 py-3">
        <p class="hidden text-sm text-muted sm:block">Changes apply only to this database branch.</p>
        <div class="ml-auto flex gap-3">
          <UButton variant="ghost" color="neutral" :to="backUrl()">Cancel</UButton>
          <UButton :loading="saving" @click="save">Save</UButton>
        </div>
      </div>
    </template>
  </div>
</template>
