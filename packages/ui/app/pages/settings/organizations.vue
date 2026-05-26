<script setup lang="ts">
const store = useStore();
if (!store.user) {
  throw createError('User not found in store');
}

const tab = ref('organizations');
const tabs = ref([
  {
    label: 'Organizations',
    value: 'organizations',
    slot: 'organizations',
  },
  {
    label: 'Invitations',
    value: 'invitations',
    slot: 'invitations',
  },
]);
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div class="w-full border border-default rounded-lg p-6 space-y-6">
      <div>
        <p class="font-semibold">Manage Organizations</p>
        <p class="text-sm text-muted">Manage which organization you are currently active in.</p>
      </div>
    </div>
    <UTabs v-model="tab" :items="tabs" class="min-h-32">
      <template #organizations>
        <dashboard-organizations />
      </template>
      <template #invitations>
        <dashboard-invitations />
      </template>
    </UTabs>
  </div>
</template>
