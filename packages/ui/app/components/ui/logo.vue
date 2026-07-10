<script setup lang="ts">
  withDefaults(defineProps<{
    collapsed?: boolean
  }>(), {
    collapsed: false,
  });
</script>

<template>
  <div class="flex justify-center items-center">
    <div class="size-8 rounded-md bg-primary flex items-center justify-center shrink-0">
      <svg class="size-5 text-white" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
        <path d="M12 4L4 8L12 12L20 8L12 4Z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
        <path d="M4 16L12 20L20 16" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
        <path d="M4 12L12 16L20 12" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
      </svg>
    </div>
    <span
      v-if="!collapsed"
      class="font-semibold tracking-tight text-base ml-2.5"
    >CPlane</span>
  </div>
</template>
