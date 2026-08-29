<script setup lang="ts">
import { COMPUTE_UNIT_ITEMS, computeUnitByLabel } from '~/utils/compute-units'
import { ICONS } from '~/utils/icons'
import { getErrorMessage } from '~/utils/errors'

const store = useStore()
const route = useRoute()
const toast = useToast()

const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() || null)
const projectName = computed(() => store.projects.find(p => p.id === projectId.value)?.name ?? projectId.value ?? '')

const loading = ref(false)
const error = ref('')

const name = ref('')
const computeUnit = ref('0.5')
const backupRetentionDays = ref<number | null>(30)
const highAvailability = ref(false)
const readReplicas = ref(2)

async function handleCreate() {
  if (!orgId.value || !projectId.value || !name.value.trim()) return
  loading.value = true; error.value = ''
  const unit = computeUnitByLabel(computeUnit.value)
  try {
      await cplaneFetch(`/api/organization/${orgId.value as ':organization_id'}/databases/postgres` as const, {
      method: 'POST',
      body: {
        name: name.value.trim(), project_id: projectId.value,
        backup_retention_days: backupRetentionDays.value,
        cpu: `${unit?.cpu ?? 0.5}`, ram: `${Math.round((unit?.ramGib ?? 1) * 1024)}Mi`,
        high_availability: highAvailability.value,
        read_replicas: highAvailability.value ? readReplicas.value : null,
      },
    })
    toast.add({ title: 'Database created', color: 'success' })
    navigateTo(`/${route.params.organization_slug}/databases/postgres/${projectId.value}`)
  } catch (e: unknown) {
    error.value = getErrorMessage(e, 'Failed to create database')
    toast.add({ title: 'Failed to create database', color: 'error' })
  } finally { loading.value = false }
}

function backUrl() { return `/${route.params.organization_slug}/databases/postgres/${projectId.value}` }
</script>

<template>
  <div class="w-full max-w-[1200px] mx-auto">
    <header class="border-b border-default/60 pb-5">
      <UiBackLink :label="projectName" :to="backUrl()" />
      <UiPageEyebrow label="Storage &amp; Databases" />
      <h1 class="mt-2 text-2xl font-semibold">New Database</h1>
      <p class="mt-1 text-sm text-muted">Create a D1 - Postgres database.</p>
    </header>

    <div class="grid lg:grid-cols-[minmax(0,1fr)_280px]">
      <main class="divide-y divide-default/60 lg:pr-8">
        <section class="grid gap-4 py-8 lg:grid-cols-[190px_minmax(0,1fr)]"><div><h2 class="text-sm font-semibold">Database</h2><p class="mt-1 text-xs text-muted">Choose a stable resource name.</p></div><UFormField label="Name"><UInput v-model="name" placeholder="orders-db" class="w-full" :disabled="loading" /></UFormField></section>
        <section class="grid gap-4 py-8 lg:grid-cols-[190px_minmax(0,1fr)]"><div><h2 class="text-sm font-semibold">Compute</h2><p class="mt-1 text-xs text-muted">CPU and RAM scale together.</p></div><UFormField label="Compute Unit" description="1 CU = 1 vCPU + 2 GB RAM. Scale from 0.25 to 32 CU."><USelect v-model="computeUnit" :items="COMPUTE_UNIT_ITEMS" class="w-full" /></UFormField></section>
        <section class="grid gap-4 py-8 lg:grid-cols-[190px_minmax(0,1fr)]"><div><h2 class="text-sm font-semibold">Backup retention</h2><p class="mt-1 text-xs text-muted">Recovery window for this database's main branch.</p></div><UFormField label="Retention period"><USelect v-model="backupRetentionDays" :items="[{ label: '1 day', value: 1 }, { label: '7 days', value: 7 }, { label: '30 days', value: 30 }]" class="w-full" /></UFormField></section>
        <section class="grid gap-4 py-8 lg:grid-cols-[190px_minmax(0,1fr)]"><div><h2 class="text-sm font-semibold">High Availability</h2><p class="mt-1 text-xs text-muted">Add replicas for resilience.</p></div><div class="space-y-4"><UFormField label="Availability mode" description="Standard: single node. HA: multi-node with automatic failover."><div class="mt-1 grid grid-cols-2 gap-2"><button type="button" class="flex flex-col items-start gap-0.5 rounded-md border-2 p-3 text-left transition-colors" :class="!highAvailability ? 'border-primary bg-primary/10' : 'border-default/40 hover:border-default/60'" @click="highAvailability = false"><span class="text-sm font-semibold">Standard</span><span class="text-xs text-muted">Single database node</span></button><button type="button" class="flex flex-col items-start gap-0.5 rounded-md border-2 p-3 text-left transition-colors" :class="highAvailability ? 'border-primary bg-primary/10' : 'border-default/40 hover:border-default/60'" @click="highAvailability = true"><span class="text-sm font-semibold">High Availability</span><span class="text-xs text-muted">Multi-node failover</span></button></div></UFormField><UFormField v-if="highAvailability" label="Read replicas" description="Number of read-only replicas for query distribution."><UInputNumber v-model="readReplicas" :min="1" :step="1" class="w-full" /></UFormField></div></section>
        <p v-if="error" class="py-4 text-sm text-error">{{ error }}</p>
      </main>

      <aside class="border-t border-default/60 py-8 lg:border-l lg:border-t-0 lg:pl-6">
        <div class="sticky top-6 rounded-lg border border-dashed border-default p-5"><h2 class="text-sm font-semibold">Database Summary</h2><dl class="mt-5 space-y-4 text-sm"><div><dt class="text-xs text-muted">Project</dt><dd class="mt-1">{{ projectName }}</dd></div><div><dt class="text-xs text-muted">Name</dt><dd class="mt-1 font-mono text-xs">{{ name || 'Not set' }}</dd></div><div><dt class="text-xs text-muted">Compute</dt><dd class="mt-1">{{ computeUnit }}</dd></div><div><dt class="text-xs text-muted">Availability</dt><dd class="mt-1">{{ highAvailability ? `${readReplicas} read replicas` : 'Standard' }}</dd></div></dl><div class="mt-8 flex gap-3"><UButton variant="ghost" color="neutral" :to="backUrl()">Cancel</UButton><UButton :icon="ICONS.plus" :loading="loading" :disabled="!name.trim()" @click="handleCreate">Create Database</UButton></div></div>
      </aside>
    </div>
  </div>
</template>
