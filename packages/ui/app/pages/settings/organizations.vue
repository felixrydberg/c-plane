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
  <div class="w-full">
    <UPageCard
      title="Manage Organizations"
      description="Manage which organization you are currently active in."
      variant="naked"
      class="mb-4"
    />
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
