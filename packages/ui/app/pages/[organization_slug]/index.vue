<script setup lang="ts">
const store = useStore();
if (!store.organization) {
  throw createError('Organization not found');
}
</script>

<template>
  <UDashboardPanel id="files">
    <template #body/>
  </UDashboardPanel>
</template>
