<script setup lang="ts">
import { ICONS } from '~/utils/icons'

interface BranchInfo {
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

type ProjectBranch = { id: string; name: string }

const props = defineProps<{
  organizationId: string
  databaseId: string
  databaseName: string
  projectId: string
  projectName: string
  defaultBranchId: string | null
}>()

const emit = defineEmits<{ deleted: [] }>()

const toast = useToast()
const route = useRoute()

const branches = ref<(BranchInfo & { _name: string })[]>([])
const projectBranches = ref<ProjectBranch[]>([])
const branchesLoading = ref(false)

const deleteModalOpen = ref(false)
const linkBranchModalOpen = ref(false)
const deleting = ref(false)
const busy = ref(false)
const unlinkTarget = ref<(BranchInfo & { _name: string }) | null>(null)

const unlinkModalOpen = computed({
  get: () => !!unlinkTarget.value,
  set: (v) => { if (!v) unlinkTarget.value = null },
})

function confirmUnlink() {
  if (!unlinkTarget.value) return
  unlinkBranch(unlinkTarget.value).then(() => unlinkTarget.value = null)
}

const linkedBranchIds = computed(() => new Set(branches.value.map(b => b.branch_id)))
const unlinkedBranches = computed(() => projectBranches.value.filter(pb => !linkedBranchIds.value.has(pb.id)))

async function fetchBranches() {
  branchesLoading.value = true
  try {
    const [data, projBranches] = await Promise.all([
      $fetch<BranchInfo[]>(
        `/api/backend/organization/${props.organizationId}/databases/stateful/${props.databaseId}/branches`
      ),
      $fetch<ProjectBranch[]>(
        `/api/backend/organization/${props.organizationId}/projects/${props.projectId}/branches`
      ),
    ])
    projectBranches.value = projBranches
    branches.value = data.map(b => ({
      ...b,
      _name: projBranches.find(pb => pb.id === b.branch_id)?.name ?? b.branch_id,
    }))
  } catch {
    branches.value = []
  } finally {
    branchesLoading.value = false
  }
}

async function linkBranch(pb: ProjectBranch) {
  linkBranchModalOpen.value = false
  busy.value = true
  try {
    const created = await $fetch<BranchInfo>(
      `/api/backend/organization/${props.organizationId}/databases/stateful/${props.databaseId}/branches`,
      { method: 'POST', body: { branch_id: pb.id } }
    )
    branches.value = [...branches.value, { ...created, _name: pb.name }]
    toast.add({ title: `Linked ${pb.name}`, color: 'success' })
  } catch {
    toast.add({ title: 'Failed to link branch', color: 'error' })
  } finally {
    busy.value = false
  }
}

onMounted(() => { fetchBranches() })

async function unlinkBranch(b: BranchInfo & { _name: string }) {
  busy.value = true
  try {
    await $fetch(
      `/api/backend/organization/${props.organizationId}/databases/stateful/${props.databaseId}/branches/${b.branch_id}`,
      { method: 'DELETE' }
    )
    branches.value = branches.value.filter(br => br.id !== b.id)
    toast.add({ title: `Deleted ${b._name}`, color: 'success' })
  } catch {
    toast.add({ title: 'Failed to unlink branch', color: 'error' })
  } finally {
    busy.value = false
  }
}


async function handleDelete() {
  deleting.value = true
  try {
    await $fetch(
      `/api/backend/organization/${props.organizationId}/databases/stateful/${props.databaseId}`,
      { method: 'DELETE' }
    )
    deleteModalOpen.value = false
    emit('deleted')
  } catch {
    toast.add({ title: 'Failed to delete database', color: 'error' })
  } finally {
    deleting.value = false
  }
}


function parseCpu(cpu: string | null): number { return cpu ? Number.parseFloat(cpu) || 0.5 : 0.5 }
function parseRamGib(ram: string | null): number {
  if (!ram) return 1
  const m = ram.match(/^(\d+(?:\.\d+)?)\s*Mi$/i)
  return m ? Math.round(Number.parseFloat(m[1]) / 1024 * 100) / 100 : Number.parseFloat(ram) || 1
}

const hasHa = computed(() => branches.value.some(b => b.high_availability))
const defaultBranchId = computed(() => props.defaultBranchId)

function isDefaultBranch(b: BranchInfo): boolean {
  return defaultBranchId.value !== null && b.id === defaultBranchId.value
}

</script>

<template>
  <div class="bg-default rounded-lg border border-default/60 overflow-hidden">
    <!-- Header -->
    <div class="flex items-center justify-between px-5 py-4 border-b border-default/30">
      <div class="flex items-center gap-3 min-w-0">
        <div class="min-w-0">
          <span class="block text-sm font-semibold truncate">{{ databaseName }}</span>
          <span class="mt-0.5 block text-xs text-muted">Postgres database</span>
        </div>
        <span
          v-if="hasHa"
          class="inline-flex items-center gap-1 text-[10px] font-medium text-emerald-600 dark:text-emerald-400"
        >
          <span class="size-1.5 rounded-full bg-emerald-500" />
          HA
        </span>
      </div>
      <div class="flex items-center gap-2 shrink-0">
        <span class="text-[11px] text-muted tabular-nums">
          {{ branches.length }} branch{{ branches.length !== 1 ? 'es' : '' }}
        </span>
        <UButton
          v-if="unlinkedBranches.length > 0"
          variant="solid"
          size="xs"
          color="neutral"
          :icon="ICONS.link"
          @click="linkBranchModalOpen = true"
        >
          Link
        </UButton>
        <UButton
          variant="solid"
          size="xs"
          color="error"
          :icon="ICONS.trash"
          :loading="deleting"
          @click="deleteModalOpen = true"
        >
          Delete
        </UButton>
      </div>
    </div>

    <!-- Branches -->
    <div v-if="branchesLoading" class="px-5 py-6 text-center text-sm text-muted">Loading branches&hellip;</div>
    <template v-else-if="branches.length > 0">
      <NuxtLink
        v-for="b in branches"
        :key="b.id"
        :to="`/${route.params.organization_slug}/databases/stateful/${projectId}/${databaseId}/${b.branch_id}`"
        class="group flex items-center gap-4 px-5 py-4 hover:bg-elevated/50 transition-colors border-b border-default/10 last:border-b-0"
      >
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <span class="text-sm font-medium truncate">{{ b._name }}</span>
            <span v-if="isDefaultBranch(b)" class="rounded-full bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary shrink-0">Default</span>
          </div>
          <div class="flex items-center gap-2 mt-2 text-[11px] text-muted font-mono">
            <span>{{ b.cpu ? `${parseCpu(b.cpu)}c` : '0.5c' }}</span>
            <span class="text-default/20">&bull;</span>
            <span>{{ b.ram ? `${parseRamGib(b.ram)}G` : '1G' }}</span>
            <template v-if="b.high_availability">
              <span class="text-default/20">&bull;</span>
              <span class="text-emerald-600 dark:text-emerald-400">HA</span>
            </template>
            <template v-if="b.read_replicas">
              <span class="text-default/20">&bull;</span>
              <span>{{ b.read_replicas }} repl</span>
            </template>
            <template v-if="b.autoscaling_enabled">
              <span class="text-default/20">&bull;</span>
              <span class="text-violet-600 dark:text-violet-400">auto</span>
            </template>
          </div>
        </div>

        <UIcon name="i-heroicons:chevron-right" class="size-4 text-muted opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />

        <UButton
          v-if="!isDefaultBranch(b)"
          variant="solid"
          size="xs"
          color="error"
          :icon="ICONS.trash"
          :loading="busy && unlinkTarget?.id === b.id"
          @click.prevent.stop="unlinkTarget = b"
        >
          Delete
        </UButton>
      </NuxtLink>
    </template>
    <div v-else class="px-5 py-12 text-center">
      <p class="text-sm font-medium">No branches linked</p>
      <p class="mt-1 text-sm text-muted">Link a project branch to configure this database for it.</p>
    </div>
  </div>

  <UModal v-model:open="unlinkModalOpen" title="Delete Branch">
    <template #body>
      <p class="text-sm">Delete <strong>{{ unlinkTarget?._name }}</strong> from {{ databaseName }}? The branch will still exist in the project.</p>
    </template>
    <template #footer>
      <div class="flex justify-end gap-3">
        <UButton variant="ghost" color="neutral" @click="unlinkTarget = null">Cancel</UButton>
        <UButton color="error" :icon="ICONS.trash" :loading="busy" @click="confirmUnlink">Delete</UButton>
      </div>
    </template>
  </UModal>


  <UModal v-model:open="linkBranchModalOpen" title="Link Branch" :ui="{ content: 'max-w-sm' }">
    <template #body>
      <p class="text-sm text-muted mb-4">Choose a project branch to link {{ databaseName }} to.</p>
      <div class="border border-default/40 rounded-lg overflow-hidden">
        <button
          v-for="pb in unlinkedBranches"
          :key="pb.id"
          class="w-full flex items-center gap-3 px-4 py-2.5 text-left hover:bg-elevated/60 transition-colors border-b border-default/10 last:border-b-0"
          @click="linkBranch(pb)"
        >
          <UIcon name="i-heroicons:folder" class="size-4 text-muted shrink-0" />
          <span class="text-sm">{{ pb.name }}</span>
          <UIcon name="i-heroicons:chevron-right" class="size-4 text-muted ml-auto" />
        </button>
      </div>
    </template>
    <template #footer>
      <div class="flex justify-end">
        <UButton variant="ghost" color="neutral" @click="linkBranchModalOpen = false">Cancel</UButton>
      </div>
    </template>
  </UModal>

  <UModal v-model:open="deleteModalOpen" title="Delete Database" description="This action cannot be undone.">
    <template #body>
      <div class="space-y-4">
        <p class="text-sm">Are you sure you want to delete <strong>{{ databaseName }}</strong>?</p>
        <div class="rounded-lg border border-warning bg-warning/5 p-3 text-sm text-warning space-y-1">
          <p>This will permanently remove:</p>
          <ul class="list-disc list-inside space-y-0.5">
            <li>The database and all its data</li>
            <li>All backups associated with this database</li>
            <li>All database branches across every project branch</li>
          </ul>
        </div>
      </div>
    </template>
    <template #footer>
      <div class="flex justify-end gap-3">
        <UButton variant="ghost" color="neutral" :disabled="deleting" @click="deleteModalOpen = false">Cancel</UButton>
        <UButton color="error" :icon="ICONS.trash" :loading="deleting" @click="handleDelete">Delete</UButton>
      </div>
    </template>
  </UModal>
</template>
