<script setup lang="ts" generic="T">
import type { TableColumn } from '@nuxt/ui';

const props = defineProps({
  items: {
    type: Array as PropType<Array<T>>,
    required: true,
  },
  status: {
    type: String,
    required: true,
  },
  columns: {
    type: Array as PropType<Array<TableColumn<T>>>,
    required: true,
  }
})
</script>

<template>
  <UPageCard>
    <UTable
      :columns="columns"
      :data="items"
      :status="status"
    />
  </UPageCard>
</template>
