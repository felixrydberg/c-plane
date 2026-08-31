<script setup lang="ts">
defineProps<{ label: string }>()
</script>

<template>
  <p class="text-sm font-medium text-primary">{{ label }}</p>
</template>
