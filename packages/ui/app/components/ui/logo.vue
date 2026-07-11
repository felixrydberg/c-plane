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
    <div class="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary shadow-sm shadow-primary/20 ring-1 ring-primary/20">
      <svg class="size-5 text-white" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <path d="M19.5 7.75 12 4 4.5 7.75 12 11.5l4.5-2.25" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round" />
        <path d="m4.5 12 7.5 3.75 4.5-2.25" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round" />
        <path d="m4.5 16.25 7.5 3.75 7.5-3.75" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round" />
        <circle cx="19.5" cy="12" r="1.25" fill="currentColor" />
      </svg>
    </div>
    <span
      v-if="!collapsed"
      class="ml-3 truncate text-lg font-semibold tracking-[-0.035em] text-highlighted"
    >C<span class="text-primary">Plane</span></span>
  </div>
</template>
