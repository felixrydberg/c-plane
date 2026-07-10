<script setup lang="ts">
import { COMPUTE_UNIT_ITEMS, computeUnitByLabel } from '~/utils/compute-units'

const store = useStore()
const route = useRoute()
const toast = useToast()

const orgId = computed(() => store.organization?.id ?? '')
const projectId = computed(() => route.params.project_id?.toString() || null)
const projectName = computed(() => store.projects.find(p => p.id === projectId.value)?.name ?? projectId.value ?? '')

const loading = ref(false)
const error = ref('')

const name = ref('')
const computeUnit = ref('XS')
const highAvailability = ref(false)
const readReplicas = ref(2)

async function handleCreate() {
  if (!orgId.value || !projectId.value || !name.value.trim()) return
  loading.value = true; error.value = ''
  const unit = computeUnitByLabel(computeUnit.value)
  try {
    await $fetch(`/api/backend/organization/${orgId.value}/databases/stateful`, {
      method: 'POST',
      body: {
        name: name.value.trim(), project_id: projectId.value,
        cpu: `${unit?.cpu ?? 0.5}`, ram: `${Math.round((unit?.ramGib ?? 1) * 1024)}Mi`,
        high_availability: highAvailability.value,
        read_replicas: highAvailability.value ? readReplicas.value : null,
      },
    })
    toast.add({ title: 'Database created', color: 'success' })
    navigateTo(`/${route.params.organization_slug}/databases/stateful/${projectId.value}`)
  } catch (e: unknown) {
    error.value = (e as any)?.data?.message ?? 'Failed to create database'
    toast.add({ title: 'Failed to create database', color: 'error' })
  } finally { loading.value = false }
}

function backUrl() { return `/${route.params.organization_slug}/databases/stateful/${projectId.value}` }
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div>
      <UiBackLink :label="projectName" :to="backUrl()" />
      <h1 class="text-2xl font-semibold">New Database</h1>
      <p class="text-muted text-sm mt-1">Create a new stateful Postgres database.</p>
    </div>

    <div class="w-full border border-default rounded-lg">
      <div class="px-4 py-3 border-b border-default"><p class="text-sm font-semibold">Database</p></div>
      <div class="p-4">
        <div class="flex items-center gap-4 px-1"><span class="w-28 text-sm">Name</span><UInput v-model="name" placeholder="e.g. my-database" class="w-64" size="sm" :disabled="loading" /></div>
      </div>
    </div>

    <div class="w-full border border-default rounded-lg">
      <div class="px-4 py-3 border-b border-default"><p class="text-sm font-semibold">Compute</p><p class="mt-0.5 text-xs text-muted">CPU and RAM scale together as a compute unit.</p></div>
      <div class="p-4 space-y-3">
        <div class="flex items-center gap-4 px-1"><span class="w-28 text-sm">Compute Unit</span><div class="w-28"><USelect v-model="computeUnit" :items="COMPUTE_UNIT_ITEMS" size="sm" class="w-full" /></div></div>
      </div>
    </div>

    <div class="w-full border border-default rounded-lg">
      <div class="px-4 py-3 border-b border-default"><p class="text-sm font-semibold">High Availability</p></div>
      <div class="p-4 space-y-3">
        <UCheckbox v-model="highAvailability" label="Enable high availability" />
        <template v-if="highAvailability">
          <div class="flex items-center gap-4 px-1"><span class="w-28 text-sm">Replicas</span><UInput v-model.number="readReplicas" type="number" :min="2" size="sm" class="w-28" /><span class="text-xs text-muted">read replicas (min 2)</span></div>
        </template>
      </div>
    </div>

    <p v-if="error" class="text-sm text-error">{{ error }}</p>

    <div class="flex justify-end gap-3">
      <UButton variant="ghost" color="neutral" :to="backUrl()">Cancel</UButton>
      <UButton :loading="loading" :disabled="!name.trim()" @click="handleCreate">Continue</UButton>
    </div>
  </div>
</template>
