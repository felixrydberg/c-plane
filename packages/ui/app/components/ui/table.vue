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
    <div class="rounded-lg">
      <component :is="tableWrapperComponent" :items="props.getContextMenuItems ? contextItems : undefined">
        <UTable
          ref="table"
          :data="items"
          :columns="_columns"
          :loading="props.status === 'pending'"
          class="flex-1"
          :ui="{
            base: 'border-separate border-spacing-y-2',
            thead: '[&>tr>th]:py-1 [&>tr>th]:px-4 [&>tr>th]:text-sm',
            tbody: '[&>tr]:data-[selectable=true]:hover:bg-transparent',
            tr: 'group cursor-pointer h-10 max-h-10',
            td: 'py-2 px-4 bg-elevated/50 group-hover:bg-elevated transition-colors border-y border-default first:rounded-l-lg last:rounded-r-lg first:border-l last:border-r empty:hidden text-sm',
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
