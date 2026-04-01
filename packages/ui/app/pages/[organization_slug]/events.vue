<script setup lang="ts">
import type { CalendarDate } from '@internationalized/date';
import { event_types_name } from '~~/shared/utils/events';
import type { event_types } from '~~/server/schema';
import type { TableColumn } from '@nuxt/ui';

const store = useStore();

if (!store.organization?.id) {
  throw createError('Organization not found in store');
}

const NuxtTime = resolveComponent("NuxtTime");
const UBadge = resolveComponent("UBadge");

const limit = 50;
const offset = ref(0);

type EventType = typeof event_types.enumValues[number];

const eventTypeOptions = (Object.entries(event_types_name) as [EventType, string][]).map(([value, label]) => ({
  label,
  value,
}));

const selectedTypes = ref<{ label: string; value: EventType }[]>([]);
const dateRange = shallowRef<{ start: CalendarDate | undefined; end: CalendarDate | undefined }>({ start: undefined, end: undefined });

const { data, status } = await useFetch(
  () => `/api/organization/${store.organization?.id as ':organization_id'}/events`,
  {
    query: computed(() => ({
      limit,
      offset: offset.value,
      ...(selectedTypes.value.length > 0 ? { type: selectedTypes.value.map(t => t.value) } : {}),
      ...(dateRange.value.start ? { from: dateRange.value.start.toDate('UTC').toISOString() } : {}),
      ...(dateRange.value.end ? { to: new Date(dateRange.value.end.toDate('UTC').setHours(23, 59, 59, 999)).toISOString() } : {}),
    })),
    watch: [offset, selectedTypes, dateRange],
  }
);

const events = computed(() => data.value?.data ?? []);
const total = computed(() => data.value?.pagination.total ?? 0);

type Event = NonNullable<typeof data.value>['data'][number];

const columns: TableColumn<Event>[] = [
  {
    accessorKey: 'created_at',
    header: 'Time',
    meta: { class: { th: 'w-32 whitespace-nowrap', td: 'w-32 whitespace-nowrap' } },
    cell: (item) => h(NuxtTime, {
      class: 'text-xs text-muted',
      datetime: item.row.original.created_at,
      locale: 'en',
      dateStyle: 'short',
      timeStyle: 'short',
      hour12: false,
    }),
  },
  {
    accessorKey: 'type',
    header: 'Event',
    meta: { class: { th: 'w-56 whitespace-nowrap', td: 'w-56 whitespace-nowrap' } },
    cell: (item) => h('code', { class: 'text-xs' }, event_types_name[item.row.original.type as EventType]),
  },
  {
    id: 'system',
    header: '',
    meta: { class: { th: 'w-20', td: 'w-20' } },
    cell: (item) => h(UBadge, {
      label: item.row.original.system ? 'System' : 'User',
      color: 'info', variant: 'soft', size: 'sm'
    })
  },
  {
    id: 'filler',
    enableHiding: false,
    header: '',
    meta: { class: { th: 'w-full', td: 'w-full' } },
  },
];

const formatPayload = (payload: Record<string, unknown>) => {
  return JSON.stringify(payload, null, 2);
};

const clearFilters = () => {
  selectedTypes.value = [];
  dateRange.value = { start: undefined, end: undefined };
  offset.value = 0;
};

const hasFilters = computed(() => selectedTypes.value.length > 0 || dateRange.value.start !== undefined || dateRange.value.end !== undefined);
</script>

<template>
  <UDashboardPanel id="events">
    <template #body>
      <UiPageContainer title="Events" description="View organization events and audit logs">
        <UiTable
          v-model:offset="offset"
          :columns="columns"
          :items="events"
          :status="status"
          pagination
          :total="total"
          :limit="limit"
          @select="(row) => row.toggleExpanded()"
        >
          <template #filters>
            <USelectMenu
              v-model="selectedTypes"
              :items="eventTypeOptions"
              multiple
              placeholder="Filter by event type"
              class="w-64"
              @update:model-value="offset = 0"
            />
            <UInputDate
              v-model="dateRange"
              range
              variant="soft"
              @update:model-value="offset = 0"
            />
            <UButton
              v-if="hasFilters"
              variant="soft"
              color="neutral"
              size="sm"
              leading-icon="i-heroicons:x-mark"
              @click="clearFilters"
            >
              Clear
            </UButton>
          </template>

          <template #expanded="{ row }">
            <pre class="text-xs overflow-x-auto p-1"><code>{{ formatPayload(row.original.payload as Record<string, unknown>) }}</code></pre>
          </template>
        </UiTable>
      </UiPageContainer>
    </template>
  </UDashboardPanel>
</template>
