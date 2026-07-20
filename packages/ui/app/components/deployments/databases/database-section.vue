<script setup lang="ts">
import type { DatabaseBranch, Environment } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'

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

const branches = ref<(DatabaseBranch & { _name: string })[]>([])
const projectEnvironments = ref<Environment[]>([])
const branchesLoading = ref(false)

const deleteModalOpen = ref(false)
const linkBranchModalOpen = ref(false)
const deleting = ref(false)
const busy = ref(false)
const unlinkTarget = ref<(DatabaseBranch & { _name: string }) | null>(null)

const unlinkModalOpen = computed({
  get: () => !!unlinkTarget.value,
  set: (v) => { if (!v) unlinkTarget.value = null },
})

function confirmUnlink() {
  if (!unlinkTarget.value) return
  unlinkBranch(unlinkTarget.value).then(() => unlinkTarget.value = null)
}

const linkedBranchIds = computed(() => new Set(branches.value.map(b => b.branch_id)))
const unlinkedEnvironments = computed(() => projectEnvironments.value.filter(environment => !linkedBranchIds.value.has(environment.id)))

async function fetchBranches() {
  branchesLoading.value = true
  try {
    const [data, projectEnvironmentsList] = await Promise.all([
      $fetch(`/api/cplane/organization/${props.organizationId as ':organization_id'}/databases/postgres/${props.databaseId as ':database_id'}/branches` as const),
      $fetch(`/api/cplane/organization/${props.organizationId as ':organization_id'}/projects/${props.projectId as ':project_id'}/environments` as const),
    ])
    projectEnvironments.value = projectEnvironmentsList
    branches.value = data.map(b => ({
      ...b,
      _name: projectEnvironmentsList.find(environment => environment.id === b.branch_id)?.name ?? b.branch_id,
    }))
  } catch {
    branches.value = []
  } finally {
    branchesLoading.value = false
  }
}

async function linkBranch(pb: Environment) {
  linkBranchModalOpen.value = false
  busy.value = true
  try {
    const created = await $fetch(
      `/api/cplane/organization/${props.organizationId as ':organization_id'}/databases/postgres/${props.databaseId as ':database_id'}/branches` as const,
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

async function unlinkBranch(b: DatabaseBranch & { _name: string }) {
  busy.value = true
  try {
    await $fetch(`/api/cplane/organization/${props.organizationId as ':organization_id'}/databases/postgres/${props.databaseId as ':database_id'}/branches/${b.branch_id as ':branch_id'}` as const, { method: 'DELETE' })
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
    await $fetch(`/api/cplane/organization/${props.organizationId as ':organization_id'}/databases/postgres/${props.databaseId as ':database_id'}` as const, { method: 'DELETE' })
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

function isDefaultBranch(b: DatabaseBranch): boolean {
  return defaultBranchId.value !== null && b.id === defaultBranchId.value
}

</script>

<template>
  <section class="border-b border-default/60 bg-transparent last:border-b-0">
    <!-- Header -->
    <div class="flex items-center justify-between px-5 py-4 border-b border-default/50">
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
          v-if="unlinkedEnvironments.length > 0"
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
        :to="`/${route.params.organization_slug}/databases/postgres/${projectId}/${databaseId}/${b.branch_id}`"
        class="group grid gap-3 border-b border-default/30 px-5 py-4 transition-colors hover:bg-elevated/50 last:border-b-0 lg:grid-cols-[minmax(0,1.7fr)_90px_90px_100px_100px_auto] lg:items-center"
      >
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <span class="text-sm font-medium truncate">{{ b._name }}</span>
            <span v-if="isDefaultBranch(b)" class="rounded-full bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary shrink-0">Default</span>
          </div>
        </div>
        <span class="font-mono text-xs text-muted">{{ b.cpu ? `${parseCpu(b.cpu)}c` : '0.5c' }}</span>
        <span class="font-mono text-xs text-muted">{{ b.ram ? `${parseRamGib(b.ram)}G` : '1G' }}</span>
        <span class="text-xs text-muted">{{ b.high_availability ? 'HA' : 'Standard' }}</span>
        <span class="text-xs text-muted">{{ b.autoscaling_enabled ? 'Autoscaling' : `${b.read_replicas ?? 0} replicas` }}</span>

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
  <p class="mt-1 text-sm text-muted">Link a project environment to configure this database for it.</p>
    </div>
  </section>

  <UModal v-model:open="unlinkModalOpen" title="Delete Branch">
    <template #body>
      <p class="text-sm">Delete <strong>{{ unlinkTarget?._name }}</strong> from {{ databaseName }}? The branch will still exist in the project.</p>
      <div class="flex justify-end gap-3 pt-4">
        <UButton variant="ghost" color="neutral" @click="unlinkTarget = null">Cancel</UButton>
        <UButton color="error" :icon="ICONS.trash" :loading="busy" @click="confirmUnlink">Delete</UButton>
      </div>
    </template>
  </UModal>


  <UModal v-model:open="linkBranchModalOpen" title="Link Branch" :ui="{ content: 'max-w-sm' }">
    <template #body>
      <p class="text-sm text-muted mb-4">Choose a project environment to link {{ databaseName }} to.</p>
      <div class="border border-default/40 rounded-lg overflow-hidden">
        <UButton
          v-for="pb in unlinkedEnvironments"
          :key="pb.id"
          variant="solid"
          color="neutral"
          :icon="ICONS.folder"
          class="w-full justify-start rounded-none border-b border-default/10 last:border-b-0"
          @click="linkBranch(pb)"
        >
          <span class="text-sm">{{ pb.name }}</span>
        </UButton>
      </div>
      <div class="flex justify-end pt-4">
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
            <li>All database branches across every project environment</li>
          </ul>
        </div>
      </div>
      <div class="flex justify-end gap-3 pt-4">
        <UButton variant="ghost" color="neutral" :disabled="deleting" @click="deleteModalOpen = false">Cancel</UButton>
        <UButton color="error" :icon="ICONS.trash" :loading="deleting" @click="handleDelete">Delete</UButton>
      </div>
    </template>
  </UModal>
</template>
