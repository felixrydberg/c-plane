<script setup lang="ts">
  const props = withDefaults(defineProps<{
    size?: 'md' | 'lg' | 'xl' | '2xl';
  }>(), {
    size: 'md',
  });
</script>

<template>
  <div class="flex justify-start items-center gap-2.5">
    <div
      :class="{
        'size-6': props.size === 'md',
        'size-8': props.size === 'lg',
        'size-10': props.size === 'xl',
        'size-12': props.size === '2xl',
      }"
      class="rounded-md bg-primary flex items-center justify-center shrink-0"
    >
      <svg
        :class="{
          'size-4': props.size === 'md',
          'size-5': props.size === 'lg',
          'size-6': props.size === 'xl',
          'size-8': props.size === '2xl',
        }"
        viewBox="0 0 24 24"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        class="text-white"
      >
        <path d="M12 4L4 8L12 12L20 8L12 4Z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
        <path d="M4 16L12 20L20 16" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
        <path d="M4 12L12 16L20 12" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
      </svg>
    </div>
    <span
      v-if="props.size === 'lg' || props.size === 'xl' || props.size === '2xl'"
      :class="{
        'text-lg': props.size === 'lg',
        'text-xl': props.size === 'xl',
        'text-2xl': props.size === '2xl',
      }"
      class="font-semibold tracking-tight"
    >CPlane</span>
  </div>
</template>
