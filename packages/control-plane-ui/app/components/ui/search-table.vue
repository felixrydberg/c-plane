<script setup lang="ts" generic="T">
  import { refDebounced } from '@vueuse/core';
  import type { TableColumn } from '@nuxt/ui';

  const props = defineProps({
    items: {
      type: Array as PropType<Array<T>>,
      required: false,
      default: () => ([]),
    },
    pagination: {
      type: Boolean,
      required: false,
      default: false
    },
    loading: {
      type: Boolean,
      required: false,
      default: false
    },
    columns: {
      type: Array as PropType<Array<TableColumn<T>>>,
      required: true 
    },
    total: {
      type: Number,
      required: false,
      default: 0
    },
    limit: {
      type: Number,
      required: false,
      default: 10
    },
    getRowId: {
      type: Function as PropType<(row: T) => string>,
      required: false,
      default: undefined
    }
  });

  const offset = defineModel("offset", {
    type: Number,
    required: false,
    default: 0
  });

  const query = defineModel("query", {
    type: String,
    required: false,
    default: ""
  });

  const selected = defineModel("selected", {
    type: Object as PropType<{ [key: string]: boolean }>,
    required: false,
    default: () => ({})
  });

  const table = useTemplateRef('table');
  const input = ref("");
  const inputDebounced = refDebounced(input, 200);
  watch (inputDebounced, (newVal) => {
    query.value = newVal;
  });

  const page = ref(1);

  const onUpdatePage = (newPage: number) => {
    offset.value = (newPage - 1) * props.limit;
  };
</script>

<template>
  <div class="flex flex-col items-center mb-3">
    <UInput
      v-model="input"
      placeholder="Search..."
      :loading="loading"
      ignore-filter
      class="w-full"
    />
    <UTable
      ref="table"
      v-model:row-selection="selected"
      :get-row-id="getRowId"
      :columns="columns"
      :data="items"
      :loading="loading"
      class="w-full min-h-96"
    />
    <UPagination
      v-model:page="page"
      :disabled="total < limit"
      :items-per-page="limit"
      :total="total"
      @update:page="onUpdatePage"
    />
  </div>
</template>
