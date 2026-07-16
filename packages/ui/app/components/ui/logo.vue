<script setup lang="ts">
withDefaults(defineProps<{
  collapsed?: boolean
}>(), {
  collapsed: false,
})
</script>

<template>
  <div
    class="flex min-w-0 items-center"
    :class="collapsed ? 'justify-center' : 'justify-start'"
    role="img"
    aria-label="CPlane"
  >
    <span class="size-3 shrink-0 rounded-full bg-primary" aria-hidden="true" />
    <span
      v-if="!collapsed"
      class="ml-2 truncate text-xl font-normal tracking-[-0.04em] text-highlighted"
    >CPlane</span>
  </div>
</template>
