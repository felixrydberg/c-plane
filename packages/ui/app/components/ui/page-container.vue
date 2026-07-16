<script setup lang="ts">
const props = defineProps({
  title: {
    type: String,
    required: false,
    default: ''
  },
  size: {
    type: String,
    required: false,
    default: ''
  }
});
</script>

<template>
  <div class="h-full flex flex-1 grow justify-center rounded-md border border-default bg-default p-4 lg:p-6 xl:p-8">
    <div :class="`${props.size} w-full flex flex-col gap-6`">
      <h3 v-if="props.title" class="text-lg font-semibold">
        {{ props.title }}
      </h3>
      <slot />
    </div>
  </div>
</template>
