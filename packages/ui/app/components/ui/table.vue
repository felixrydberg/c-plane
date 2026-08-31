<script setup lang="ts" generic="T extends Record<string, any>">
import type { ContextMenuItem, TableColumn, TableRow } from '@nuxt/ui';
import type { PropType } from 'vue';
import { useSlots } from 'vue';

const _upperFirst = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

const props = defineProps({
  columns: {
    type: Array as PropType<TableColumn<T>[]>,
    required: true
  },
  items: {
    type: Array as PropType<T[]>,
    required: true
  },
  status: {
    type: String,
    required: true
  },
  getContextMenuItems: {
    type: Function as PropType<(row: TableRow<T>) => ContextMenuItem[]>,
    required: false,
    default: undefined,
  },
  filterClass: {
    type: String,
    required: false,
    default: 'flex gap-2'
  },
  disableHeader: {
    type: Boolean,
    required: false,
    default: false,
  },
  selectable: {
    type: Boolean,
    required: false,
    default: false,
  },
  pagination: {
    type: Boolean,
    required: false,
    default: false
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
  }
});

const UButton = resolveComponent("UButton");
const slots = useSlots();
const _columns = computed(() => {
  const columns: typeof props.columns = [...props.columns];

  if (slots.expanded) {
    columns.unshift({
      id: 'expand',
      meta: { class: { th: 'w-8', td: 'w-8 h-8 py-0' } },
      cell: ({ row }) => h(UButton, {
        variant: 'ghost',
        color: 'neutral',
        size: 'xs',
        icon: row.getIsExpanded() ? 'i-heroicons-chevron-down-20-solid' : 'i-heroicons-chevron-right-20-solid',
        onClick: row.getToggleExpandedHandler(),
      }),
    });
  }

  return columns;
});

const offset = defineModel("offset", {
  type: Number,
  required: false,
  default: 0
});

const page = ref(1);
const emits = defineEmits<{
  (e: 'select', row: TableRow<T>): void;
}>();

const onUpdatePage = (newPage: number) => {
  offset.value = (newPage - 1) * props.limit;
};

const UContextMenu = resolveComponent('UContextMenu');
const contextItems = ref<ContextMenuItem[]>([]);
const onContextMenu = (_e: Event, row: TableRow<T>) => {
  if (props.getContextMenuItems) {
    contextItems.value = props.getContextMenuItems(row);
  }
};

const table = useTemplateRef('table');
const tableWrapperComponent = computed(() => props.getContextMenuItems ? UContextMenu : 'div');
const rowClass = computed(() => props.selectable ? 'group cursor-pointer' : 'group');

defineExpose({
  table,
});
</script>

<template>
  <div class="flex flex-col gap-4 h-full">
    <div v-if="!disableHeader" class="flex items-start gap-2 overflow-x-auto">
      <div :class="filterClass">
        <slot name="filters" />
      </div>
      <UDropdownMenu
        :items="table?.tableApi?.getAllColumns().filter(column => column.getCanHide()).map(column => ({
          label: _upperFirst(column.id),
          type: 'checkbox' as const,
          checked: column.getIsVisible(),
          onUpdateChecked(checked: boolean) {
            table?.tableApi?.getColumn(column.id)?.toggleVisibility(!!checked)
          },
          onSelect(e?: Event) {
            e?.preventDefault()
          }
        }))"
        :content="{ align: 'end' }"
      >
        <UButton
          label="Columns"
          size="sm"
          variant="soft"
          trailing-icon="i-heroicons:chevron-down"
          class="ml-auto"
          aria-label="Columns select dropdown"
        />
      </UDropdownMenu>
    </div>
    <div class="overflow-x-auto rounded-lg border border-default/60 bg-default">
      <component :is="tableWrapperComponent" :items="props.getContextMenuItems ? contextItems : undefined">
        <UTable
          ref="table"
          :data="items"
          :columns="_columns"
          :loading="props.status === 'pending'"
          class="min-w-full flex-1"
          :ui="{
            base: 'border-separate border-spacing-0',
            thead: 'bg-elevated/20 [&>tr>th]:border-b [&>tr>th]:border-default/60 [&>tr>th]:px-4 [&>tr>th]:py-3 [&>tr>th]:text-left [&>tr>th]:text-xs [&>tr>th]:font-medium [&>tr>th]:text-muted',
            tbody: '[&>tr:last-child>td]:border-b-0 [&>tr]:transition-colors [&>tr]:hover:bg-elevated/20',
            tr: rowClass,
            td: 'border-b border-default/40 px-4 py-3 text-sm empty:hidden',
          }"
          @contextmenu="onContextMenu"
          @select="(e, row: TableRow<T>) => emits('select', row)"
        >
          <template #empty>
            <div class="flex items-center justify-center">
              <slot name="empty" />
            </div>
          </template>
          <template v-if="$slots.expanded" #expanded="slotProps">
            <slot name="expanded" v-bind="slotProps" />
          </template>
        </UTable>
      </component>
      <UPagination
        v-if="pagination"
        v-model:page="page"
        :disabled="total < limit"
        :items-per-page="limit"
        :total="total"
        class="mt-4 mx-auto flex justify-center items-center"
        @update:page="onUpdatePage"
      />
    </div>
  </div>
</template>
