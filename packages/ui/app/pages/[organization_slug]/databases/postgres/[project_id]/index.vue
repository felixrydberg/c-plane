<script setup lang="ts">
import { h } from 'vue'
import type { DatabaseBranch, DatabaseWithBranches, Environment } from '@cplane/sdk'
import type { TableColumn } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

type BranchRow = DatabaseBranch & {
  _defaultBranchId: string | null
  _name: string
  _rowType: 'branch'
}

type DatabaseRow = DatabaseWithBranches & {
  _rowType: 'database'
  subRows: BranchRow[]
}

type PostgresTableRow = DatabaseRow | BranchRow

const store = useStore()
const route = useRoute()
const toast = useToast()

const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() || null)
const databasesUrl = computed(() => orgId.value
  ? `/api/organization/${orgId.value as ':organization_id'}/databases/postgres` as const
  : '')

const search = ref('')
const refreshing = ref(false)
const projectEnvironments = ref<Environment[]>([])

const deleteModalOpen = ref(false)
const linkBranchModalOpen = ref(false)
const deleting = ref(false)
const busy = ref(false)
const selectedDatabase = ref<DatabaseRow | null>(null)
const selectedBranch = ref<BranchRow | null>(null)

const { data: databases, status, refresh: refreshDatabases } = await useFetch<DatabaseWithBranches[]>(databasesUrl, {
  default: () => [],
  query: { project_id: projectId },
})

const filteredDatabases = computed(() => {
  const query = search.value.trim().toLowerCase()
  return query ? databases.value.filter(database => database.name.toLowerCase().includes(query)) : databases.value
})

const databaseRows = computed<DatabaseRow[]>(() => filteredDatabases.value.map(database => ({
  ...database,
  _rowType: 'database' as const,
  subRows: database.branches.map(branch => ({
    ...branch,
    _defaultBranchId: database.default_branch_id ?? null,
    _name: projectEnvironments.value.find(environment => environment.id === branch.branch_id)?.name ?? branch.branch_id,
    _rowType: 'branch' as const,
  })),
})))

const selectedDatabaseBranches = computed(() => selectedDatabase.value
  ? databaseRows.value.find(database => database.id === selectedDatabase.value?.id)?.subRows ?? []
  : [])

const unlinkedEnvironments = computed(() => {
  const linkedBranchIds = new Set(selectedDatabaseBranches.value.map(branch => branch.branch_id))
  return projectEnvironments.value.filter(environment => !linkedBranchIds.has(environment.id))
})

const unlinkModalOpen = computed({
  get: () => !!selectedBranch.value,
  set: (value) => { if (!value) selectedBranch.value = null },
})

function isDatabaseRow(row: PostgresTableRow): row is DatabaseRow {
  return row._rowType === 'database'
}

function getSubRows(row: PostgresTableRow) {
  return isDatabaseRow(row) ? row.subRows : undefined
}

function hasHa(database: DatabaseRow) {
  return database.subRows.some(branch => branch.high_availability)
}

function isDefaultBranch(branch: BranchRow) {
  return branch._defaultBranchId !== null && branch.id === branch._defaultBranchId
}

function emptyDatabaseCell() {
  return h('span', { class: 'block h-px', 'aria-hidden': 'true' })
}

function branchUrl(branch: BranchRow) {
  return `/${route.params.organization_slug}/databases/postgres/${projectId.value}/${branch.database_id}/${branch.branch_id}`
}

function unlinkedEnvironmentsFor(database: DatabaseRow) {
  const linkedBranchIds = new Set(database.subRows.map(branch => branch.branch_id))
  return projectEnvironments.value.filter(environment => !linkedBranchIds.has(environment.id))
}

function setDatabaseBranches(databaseId: string, branches: DatabaseBranch[]) {
  databases.value = databases.value.map(database => database.id === databaseId
    ? { ...database, branches }
    : database)
}

async function fetchRelatedData() {
  if (!orgId.value || !projectId.value) return

  try {
    projectEnvironments.value = await cplaneFetch(`/api/organization/${orgId.value as ':organization_id'}/projects/${projectId.value as ':project_id'}/environments` as const)
  } catch {
    projectEnvironments.value = []
  }
}

async function reloadDatabases() {
  refreshing.value = true
  try {
    await refreshDatabases()
  } finally {
    refreshing.value = false
  }
}

function openLinkModal(database: DatabaseRow) {
  selectedDatabase.value = database
  linkBranchModalOpen.value = true
}

function openDeleteModal(database: DatabaseRow) {
  selectedDatabase.value = database
  deleteModalOpen.value = true
}

async function linkBranch(environment: Environment) {
  if (!selectedDatabase.value) return

  const database = selectedDatabase.value
  linkBranchModalOpen.value = false
  busy.value = true
  try {
    const created = await cplaneFetch(
      `/api/organization/${orgId.value as ':organization_id'}/databases/postgres/${database.id as ':database_id'}/branches` as const,
      { method: 'POST', body: { branch_id: environment.id } },
    )
    setDatabaseBranches(database.id, [...database.branches, created])
    toast.add({ title: `Linked ${environment.name}`, color: 'success' })
  } catch {
    toast.add({ title: 'Failed to link branch', color: 'error' })
  } finally {
    busy.value = false
  }
}

async function unlinkBranch(branch: BranchRow) {
  busy.value = true
  try {
    await cplaneFetch(`/api/organization/${orgId.value as ':organization_id'}/databases/postgres/${branch.database_id as ':database_id'}/branches/${branch.branch_id as ':branch_id'}` as const, { method: 'DELETE' })
    const database = databases.value.find(item => item.id === branch.database_id)
    if (database) {
      setDatabaseBranches(branch.database_id, database.branches.filter(item => item.id !== branch.id))
    }
    selectedBranch.value = null
    toast.add({ title: `Deleted ${branch._name}`, color: 'success' })
  } catch {
    toast.add({ title: 'Failed to unlink branch', color: 'error' })
  } finally {
    busy.value = false
  }
}

function confirmUnlink() {
  if (selectedBranch.value) void unlinkBranch(selectedBranch.value)
}

async function handleDelete() {
  if (!selectedDatabase.value) return

  deleting.value = true
  try {
    await cplaneFetch(`/api/organization/${orgId.value as ':organization_id'}/databases/postgres/${selectedDatabase.value.id as ':database_id'}` as const, { method: 'DELETE' })
    deleteModalOpen.value = false
    selectedDatabase.value = null
    await reloadDatabases()
  } catch {
    toast.add({ title: 'Failed to delete database', color: 'error' })
  } finally {
    deleting.value = false
  }
}

const UButton = resolveComponent('UButton')
const UDropdownMenu = resolveComponent('UDropdownMenu')
const NuxtLink = resolveComponent('NuxtLink')

const databaseColumns: TableColumn<PostgresTableRow>[] = [
  {
    id: 'name',
    header: 'Database / environment',
    cell: ({ row }) => {
      const item = row.original
      if (isDatabaseRow(item)) {
        return h('div', { class: 'min-w-0' }, [
          h('span', { class: 'block truncate font-medium' }, item.name),
          h('span', { class: 'mt-0.5 block text-xs text-muted' }, `${item.subRows.length} linked environment${item.subRows.length !== 1 ? 's' : ''}`),
          hasHa(item)
            ? h('span', { class: 'mt-1 inline-flex items-center gap-1 text-[10px] font-medium text-emerald-600 dark:text-emerald-400' }, [
                h('span', { class: 'size-1.5 rounded-full bg-emerald-500' }),
                'HA',
              ])
            : null,
        ])
      }

      return h(NuxtLink, {
        to: branchUrl(item),
        class: 'flex min-w-0 items-center gap-2 ps-4',
      }, [
        h('span', { class: 'truncate font-medium text-primary group-hover:underline group-hover:underline-offset-4' }, item._name),
        isDefaultBranch(item)
          ? h('span', { class: 'shrink-0 rounded-full bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary' }, 'Default')
          : null,
      ])
    },
  },
  {
    id: 'cpu',
    header: 'CPU',
    cell: ({ row }) => isDatabaseRow(row.original)
      ? emptyDatabaseCell()
      : h('span', { class: 'font-mono text-xs text-muted' }, row.original.cpu ? `${Number.parseFloat(row.original.cpu) || 0.5}c` : '0.5c'),
  },
  {
    id: 'ram',
    header: 'RAM',
    cell: ({ row }) => {
      if (isDatabaseRow(row.original)) return emptyDatabaseCell()
      const ram = row.original.ram
      const value = ram?.match(/^(\d+(?:\.\d+)?)\s*Mi$/i)
      const gib = value ? Math.round(Number.parseFloat(value[1]) / 1024 * 100) / 100 : Number.parseFloat(ram ?? '') || 1
      return h('span', { class: 'font-mono text-xs text-muted' }, `${gib}G`)
    },
  },
  {
    id: 'availability',
    header: 'Availability',
    cell: ({ row }) => isDatabaseRow(row.original) ? emptyDatabaseCell() : (row.original.high_availability ? 'HA' : 'Standard'),
  },
  {
    id: 'scaling',
    header: 'Scaling',
    cell: ({ row }) => isDatabaseRow(row.original)
      ? emptyDatabaseCell()
      : (row.original.autoscaling_enabled ? 'Autoscaling' : `${row.original.read_replicas ?? 0} replicas`),
  },
  {
    id: 'actions',
    header: '',
    meta: { class: { th: 'text-right', td: 'text-right' } },
    cell: ({ row }) => {
      const item = row.original
      if (isDatabaseRow(item)) {
        return h('div', { class: 'flex items-center justify-end gap-2' }, [
          unlinkedEnvironmentsFor(item).length > 0
            ? h(UButton, {
                variant: 'solid',
                color: 'neutral',
                size: 'xs',
                icon: ICONS.link,
                onClick: (event: MouseEvent) => {
                  event.stopPropagation()
                  openLinkModal(item)
                },
              }, { default: () => 'Link' })
            : null,
          h(UDropdownMenu, {
            items: [[{
              label: 'Delete',
              icon: ICONS.trash,
              color: 'error',
              onSelect: () => openDeleteModal(item),
            }]],
            size: 'sm',
            content: { align: 'end' },
          }, {
            default: () => h(UButton, {
              icon: ICONS.more,
              color: 'neutral',
              variant: 'ghost',
              size: 'xs',
              'aria-label': 'Database actions',
              onClick: (event: MouseEvent) => event.stopPropagation(),
            }),
          }),
        ])
      }

      return !isDefaultBranch(item) ? h(UButton, {
        icon: ICONS.trash,
        color: 'error',
        size: 'sm',
        loading: busy.value && selectedBranch.value?.id === item.id,
        onClick: (event: MouseEvent) => {
          event.stopPropagation()
          selectedDatabase.value = databaseRows.value.find(database => database.id === item.database_id) ?? null
          selectedBranch.value = item
        },
      }, { default: () => 'Delete' }) : null
    },
  },
]

watch([orgId, projectId], () => { void fetchRelatedData() }, { immediate: true })
watch(() => store.refreshKey, () => { void reloadDatabases() })
</script>

<template>
  <div class="flex w-full max-w-375 flex-col gap-5 mx-auto">
    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div class="min-w-0">
        <UiPageEyebrow label="Storage &amp; Databases" />
        <h1 class="mt-2 text-2xl font-semibold">Postgres</h1>
        <p class="mt-1 text-sm text-muted">Postgres databases and their linked project environments.</p>
      </div>
      <UButton class="shrink-0" :icon="ICONS.plus" color="primary" :to="`/${route.params.organization_slug}/databases/postgres/${projectId}/new`">New database</UButton>
    </div>

    <div class="flex items-center gap-2">
      <UInput
        v-model="search"
        icon="i-heroicons:magnifying-glass"
        placeholder="Search databases..."
        aria-label="Search databases"
        class="min-w-0 flex-1"
      />
      <UButton
        :icon="ICONS.refresh"
        color="neutral"
        :loading="refreshing"
        aria-label="Refresh databases"
        @click="reloadDatabases"
      >
        Refresh
      </UButton>
    </div>

    <div v-if="status === 'pending'" class="flex items-center justify-center rounded-lg border border-default/60 bg-default py-14">
      <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin text-muted" />
    </div>

    <div v-else-if="status === 'error'" class="rounded-lg border border-default/60 bg-default px-6 py-14 text-center">
      <p class="text-sm text-error">Failed to load databases.</p>
    </div>

    <UiTable
      v-else
      :status="status"
      :items="databaseRows"
      :columns="databaseColumns"
      :get-sub-rows="getSubRows"
      disable-header
    >
      <template #empty>
        <div class="flex flex-col items-center justify-center gap-3 py-14 text-center">
          <UIcon :name="ICONS.databases" class="size-10 text-muted" />
          <p class="text-muted">{{ search ? 'No matching databases.' : 'No databases yet.' }}</p>
          <p v-if="!search" class="text-sm text-dimmed">Create your first database to get started.</p>
        </div>
      </template>
    </UiTable>

    <UModal v-model:open="unlinkModalOpen" title="Delete Branch">
      <template #body>
        <p class="text-sm">Delete <strong>{{ selectedBranch?._name }}</strong> from {{ selectedDatabase?.name }}? The branch will still exist in the project.</p>
        <div class="flex justify-end gap-3 pt-4">
          <UButton variant="ghost" color="neutral" @click="selectedBranch = null">Cancel</UButton>
          <UButton color="error" :icon="ICONS.trash" :loading="busy" @click="confirmUnlink">Delete</UButton>
        </div>
      </template>
    </UModal>

    <UModal v-model:open="linkBranchModalOpen" title="Link Branch" :ui="{ content: 'max-w-sm' }">
      <template #body>
        <p class="mb-4 text-sm text-muted">Choose a project environment to link {{ selectedDatabase?.name }} to.</p>
        <div class="overflow-hidden rounded-lg border border-default/40">
          <UButton
            v-for="environment in unlinkedEnvironments"
            :key="environment.id"
            variant="solid"
            color="neutral"
            :icon="ICONS.folder"
            class="w-full justify-start rounded-none border-b border-default/10 last:border-b-0"
            @click="linkBranch(environment)"
          >
            <span class="text-sm">{{ environment.name }}</span>
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
          <p class="text-sm">Are you sure you want to delete <strong>{{ selectedDatabase?.name }}</strong>?</p>
          <div class="space-y-1 rounded-lg border border-warning bg-warning/5 p-3 text-sm text-warning">
            <p>This will permanently remove:</p>
            <ul class="list-inside list-disc space-y-0.5">
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
  </div>
</template>
