<script setup lang="ts">
import type { Database } from '@cplane/sdk'
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() || null)
const projectName = computed(() => store.projects.find(p => p.id === projectId.value)?.name ?? projectId.value ?? '')
const databasesUrl = computed(() => orgId.value
  ? `/api/organization/${orgId.value as ':organization_id'}/databases/postgres` as const
  : '')

const search = ref('')
const refreshing = ref(false)
const { data: databases, status, refresh: refreshDatabases } = await useFetch<Database[]>(databasesUrl, {
  default: () => [],
  query: { project_id: projectId },
})

const filteredDatabases = computed(() => {
  const query = search.value.trim().toLowerCase()
  return query ? databases.value.filter(database => database.name.toLowerCase().includes(query)) : databases.value
})

async function reloadDatabases() {
  refreshing.value = true
  try {
    await refreshDatabases()
  } finally {
    refreshing.value = false
  }
}

function onDatabaseDeleted() { refreshDatabases() }

watch(() => store.refreshKey, () => { refreshDatabases() })
</script>

<template>
  <div class="flex w-full max-w-375 flex-col gap-4 mx-auto">
    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div class="min-w-0">
        <UiPageEyebrow label="Storage &amp; Databases" />
        <h1 class="text-2xl font-semibold">D1 - Postgres</h1>
        <p class="mt-1 text-sm text-muted">D1 - Postgres databases and their linked project environments.</p>
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

    <div v-else-if="filteredDatabases.length > 0" class="space-y-4">
      <DeploymentsDatabasesDatabaseSection
        v-for="db in filteredDatabases"
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

    <div v-else class="flex flex-col items-center justify-center gap-3 rounded-lg border border-default/60 bg-default py-14 text-center">
      <UIcon :name="ICONS.databases" class="size-10 text-muted" />
      <p class="text-muted">{{ search ? 'No matching databases.' : 'No databases yet.' }}</p>
      <p v-if="!search" class="text-sm text-dimmed">Create your first database to get started.</p>
    </div>

  </div>
</template>
