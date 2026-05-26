<script setup lang="ts">
import type { TableColumn, ContextMenuItem, TableRow } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

interface DatabaseRow {
  id: string
  project_id: string
  name: string
  cpu: string | null
  ram: string | null
  high_availability: boolean
  read_replicas: number | null
  default_branch_id: string | null
  _projectName?: string
}

const store = useStore()
const toast = useToast()

const createModalOpen = ref(false)
const deleteModalOpen = ref(false)
const databaseToDelete = ref<DatabaseRow | null>(null)
const deleting = ref(false)

const globalListings = ref<{ projectName: string; projectId: string; databases: DatabaseRow[] }[]>([])
const globalLoading = ref(true)
const globalError = ref('')

async function fetchGlobalView() {
  if (!store.organization?.id) return
  globalLoading.value = true
  globalError.value = ''
  globalListings.value = []

  try {
    for (const project of store.projects) {
      const dbs = await $fetch<DatabaseRow[]>(
        `/api/backend/organization/${store.organization.id}/databases/stateful`,
        { query: { project_id: project.id } }
      )

      if (dbs.length > 0) {
        globalListings.value.push({
          projectName: project.name,
          projectId: project.id,
          databases: dbs.map(db => ({ ...db, _projectName: project.name })),
        })
      }
    }
  } catch {
    globalError.value = 'Failed to load databases'
  } finally {
    globalLoading.value = false
  }
}

onMounted(() => { fetchGlobalView() })

function refresh() { fetchGlobalView() }

useHead({ title: `Stateful Databases - All Projects - C-Plane` })

const columns: TableColumn<DatabaseRow>[] = [
  {
    accessorKey: 'name',
    header: 'Name',
    meta: { class: { th: 'w-48 whitespace-nowrap', td: 'w-48 whitespace-nowrap' } },
    cell: (item) => item.row.original.name,
  },
  {
    accessorKey: 'cpu',
    header: 'CPU',
    meta: { class: { th: 'w-20 whitespace-nowrap', td: 'w-20 whitespace-nowrap' } },
    cell: (item) => item.row.original.cpu ? `${item.row.original.cpu} vCPU` : '—',
  },
  {
    accessorKey: 'ram',
    header: 'RAM',
    meta: { class: { th: 'w-24 whitespace-nowrap', td: 'w-24 whitespace-nowrap' } },
    cell: (item) => item.row.original.ram ?? '—',
  },
  {
    accessorKey: 'high_availability',
    header: 'HA',
    meta: { class: { th: 'w-16 whitespace-nowrap', td: 'w-16 whitespace-nowrap' } },
    cell: (item) => item.row.original.high_availability ? 'Yes' : 'No',
  },
  {
    accessorKey: 'read_replicas',
    header: 'Replicas',
    meta: { class: { th: 'w-20 whitespace-nowrap', td: 'w-20 whitespace-nowrap' } },
    cell: (item) => item.row.original.read_replicas ?? '—',
  },
]

async function deleteDatabase() {
  const db = databaseToDelete.value
  if (!store.organization?.id || !db?.id) return
  deleting.value = true
  try {
    await $fetch(`/api/backend/organization/${store.organization.id}/databases/stateful/${db.id}`, {
      method: 'DELETE',
    })
    toast.add({ title: 'Database deleted', color: 'success' })
    deleteModalOpen.value = false
    databaseToDelete.value = null
    refresh()
  } catch {
    toast.add({ title: 'Failed to delete database', color: 'error' })
  } finally {
    deleting.value = false
  }
}

const getContextMenuItems = (row: TableRow<DatabaseRow>): ContextMenuItem[] => [
  { type: 'label' as const, label: 'Actions' },
  {
    label: 'Delete',
    icon: 'i-heroicons:trash',
    color: 'error' as const,
    onSelect: () => {
      databaseToDelete.value = row.original
      deleteModalOpen.value = true
    },
  },
]
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-semibold">Stateful Databases</h1>
        <p class="text-muted text-sm mt-1">Manage stateful Postgres databases across all projects.</p>
      </div>
      <UButton :icon="ICONS.plus" @click="createModalOpen = true">New Database</UButton>
    </div>

    <div v-if="globalLoading" class="text-center py-8"><UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" /></div>
    <p v-else-if="globalError" class="text-sm text-red-500">{{ globalError }}</p>

    <div
      v-else-if="globalListings.length === 0"
      class="flex flex-col items-center justify-center py-16 gap-3 text-center border border-dashed border-default rounded-lg"
    >
      <UIcon :name="ICONS.databases" class="size-10 text-muted" />
      <p class="text-muted">Select a project or create a database to get started.</p>
    </div>

    <template v-else>
      <div
        v-for="listing in globalListings"
        :key="listing.projectId"
        class="mb-6"
      >
        <h2 class="text-lg font-medium mb-3">{{ listing.projectName }}</h2>
        <UiTable
          :columns="columns"
          :items="listing.databases"
          status="success"
          :get-context-menu-items="getContextMenuItems"
        >
          <template #empty>
            <div class="flex flex-col items-center gap-2 py-8">
              <UIcon :name="ICONS.databases" class="size-8 text-muted" />
              <p class="text-muted text-sm">No databases found.</p>
            </div>
          </template>
        </UiTable>
      </div>
    </template>

    <DeploymentsDatabasesCreateModal
      v-model:open="createModalOpen"
      :organization-id="store.organization?.id ?? ''"
      type="stateful"
      @created="refresh"
    />

    <UModal v-model:open="deleteModalOpen" title="Delete Database" description="This action cannot be undone.">
      <template #body>
        <div class="space-y-4">
          <p class="text-sm">
            Are you sure you want to delete <strong>{{ databaseToDelete?.name }}</strong>?
          </p>
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
        <div class="flex gap-2 justify-end">
          <UButton variant="ghost" color="neutral" :disabled="deleting" @click="deleteModalOpen = false">Cancel</UButton>
          <UButton color="error" :loading="deleting" @click="deleteDatabase">Delete Database</UButton>
        </div>
      </template>
    </UModal>
  </div>
</template>
