<script setup lang="ts">
import type { Database } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const toast = useToast()
const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() || null)
const projectName = computed(() => store.projects.find(p => p.id === projectId.value)?.name ?? projectId.value ?? '')

const databases = ref<Database[]>([])
const status = ref<'pending' | 'success' | 'error' | 'idle'>('pending')

async function fetchAll() {
  if (!orgId.value || !projectId.value) return
  status.value = 'pending'
  try {
    databases.value = await $fetch(
      `/api/cplane/organization/${orgId.value as ':organization_id'}/databases/postgres` as const,
      { query: { project_id: projectId.value } }
    )
    status.value = databases.value.length > 0 ? 'success' : 'idle'
  } catch {
    status.value = 'error'
    toast.add({ title: 'Failed to load databases', color: 'error' })
  }
}

onMounted(() => { fetchAll() })

function onDatabaseDeleted() {
  fetchAll()
}

watch(() => store.refreshKey, () => { fetchAll() })
</script>

<template>
  <div class="flex w-full max-w-[1500px] flex-col gap-5 mx-auto">
    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div class="min-w-0">
        <p class="mb-2 truncate text-sm text-muted">{{ projectName }}</p>
        <h1 class="text-2xl font-semibold">Postgres Databases</h1>
        <p class="mt-1 text-sm text-muted">Postgres databases and their linked project environments.</p>
      </div>
      <UButton class="shrink-0" :icon="ICONS.plus" :to="`/${route.params.organization_slug}/databases/postgres/${projectId}/new`">New Database</UButton>
    </div>

    <div v-if="status === 'pending'" class="text-center py-8">
      <UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" />
    </div>

    <div
      v-else-if="status === 'idle'"
      class="flex flex-col items-center justify-center gap-2 px-6 py-16 text-center rounded-lg border border-dashed border-default"
    >
      <p class="text-sm font-medium">No databases in this project</p>
      <p class="text-sm text-muted">Create a database to get started.</p>
    </div>

    <p v-else-if="status === 'error'" class="text-sm text-red-500 py-8 text-center">
      Failed to load databases.
    </p>

    <div v-else class="overflow-hidden rounded-lg border border-dashed border-default bg-transparent">
      <DeploymentsDatabasesDatabaseSection
        v-for="db in databases"
        :key="db.id"
        :organization-id="orgId"
        :database-id="db.id"
        :database-name="db.name"
        :project-id="db.project_id"
        :project-name="projectName"
        :default-branch-id="db.default_branch_id"
        @deleted="onDatabaseDeleted"
      />
    </div>

  </div>
</template>
