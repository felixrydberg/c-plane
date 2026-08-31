<script setup lang="ts">
type TabItem = {
  label: string
  value: string | number
  slot?: string
  content?: string
  disabled?: boolean
}

const props = defineProps<{
  items: TabItem[]
  modelValue?: string | number
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
}>()

const internalValue = ref(props.items[0]?.value)
const activeValue = computed(() => props.modelValue ?? internalValue.value)
const activeItem = computed(() => props.items.find(item => item.value === activeValue.value) ?? props.items[0])

function selectTab(item: TabItem) {
  if (item.disabled) return
  internalValue.value = item.value
  emit('update:modelValue', item.value)
}
</script>

<template>
  <div>
    <div role="tablist" class="inline-flex max-w-full overflow-x-auto rounded-lg bg-elevated p-0.5">
      <button
        v-for="item in items"
        :key="item.value"
        type="button"
        role="tab"
        :aria-selected="item.value === activeItem?.value"
        :disabled="item.disabled"
        :class="[
          'shrink-0 rounded-md px-3 py-1.5 text-sm transition-colors focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-primary',
          item.value === activeItem?.value
            ? 'bg-default text-default shadow-sm ring-1 ring-default/70'
            : 'text-muted hover:text-default',
          item.disabled && 'cursor-not-allowed opacity-50',
        ]"
        @click="selectTab(item)"
      >
        {{ item.label }}
      </button>
    </div>

    <div v-if="activeItem" role="tabpanel" class="w-full">
      <slot :name="activeItem.slot || String(activeItem.value)">
        {{ activeItem.content }}
      </slot>
    </div>
  </div>
</template>
