<script setup lang="ts">
defineProps<{
  label: string
  to: string
}>()
</script>

<template>
  <UButton variant="link" color="neutral" class="px-0 -ml-1 mb-1" :to="to">
    <template #leading>
      <UIcon name="i-heroicons-chevron-left" class="size-4" />
    </template>
    Back to {{ label }}
  </UButton>
</template>
