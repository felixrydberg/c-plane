<script setup lang="ts">
import type { TableColumn } from '@nuxt/ui';

const store = useStore();

if (!store.organization?.id) {
  throw createError('Organization not found in store');
}

const NuxtTime = resolveComponent("NuxtTime");
const UBadge = resolveComponent("UBadge");

const limit = 50;
const offset = ref(0);

const selectedOptions = ref<string[]>([]);

const optionLabels: { value: string; label: string }[] = [
  { value: 'face_match', label: 'Face Match' },
  { value: 'liveness', label: 'Liveness' },
  { value: 'age_estimate', label: 'Age Estimate' },
];

const { data, status } = await useFetch(
  () => `/api/organization/${store.organization?.id as ':organization_id'}/sessions`,
  {
    query: computed(() => ({
      limit,
      offset: offset.value,
      ...(selectedOptions.value.includes('face_match') ? { face_match: 'true' } : {}),
      ...(selectedOptions.value.includes('liveness') ? { liveness: 'true' } : {}),
      ...(selectedOptions.value.includes('age_estimate') ? { age_estimate: 'true' } : {}),
    })),
    watch: [offset, selectedOptions],
  }
);

const sessions = computed(() => data.value?.data ?? []);
const total = computed(() => data.value?.pagination.total ?? 0);

type Session = NonNullable<typeof data.value>['data'][number];

const columns: TableColumn<Session>[] = [
  {
    accessorKey: 'created_at',
    header: 'Created At',
    meta: { class: { th: 'w-40 whitespace-nowrap', td: 'w-40 whitespace-nowrap' } },
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
    accessorKey: 'status',
    header: 'Status',
    meta: { class: { th: 'w-28 whitespace-nowrap', td: 'w-28 whitespace-nowrap' } },
    cell: (item) => h(UBadge, {
      label: item.row.original.status === 'completed'
        ? 'Completed'
        : item.row.original.status === 'pending'
          ? 'Pending'
          : item.row.original.status === 'pending_manual'
            ? 'Manual Review'
          : item.row.original.status === 'expired'
            ? 'Expired'
            : 'Pruned',
      color: item.row.original.status === 'completed'
        ? 'success'
        : item.row.original.status === 'pending'
          ? 'warning'
          : item.row.original.status === 'pending_manual'
            ? 'warning'
          : 'neutral',
      variant: 'soft',
      size: 'sm',
    }),
  },
  {
    accessorKey: 'completed_at',
    header: 'Completed At',
    meta: { class: { th: 'w-40 whitespace-nowrap', td: 'w-40 whitespace-nowrap' } },
    cell: (item) => item.row.original.completed_at
      ? h(NuxtTime, {
        class: 'text-xs text-muted',
        datetime: item.row.original.completed_at,
        locale: 'en',
        dateStyle: 'short',
        timeStyle: 'short',
        hour12: false,
      })
      : h('span', { class: 'text-xs text-muted' }, 'N/A'),
  },
  {
    accessorKey: 'verdict',
    header: 'Verdict',
    meta: { class: { th: 'w-32 whitespace-nowrap', td: 'w-32 whitespace-nowrap' } },
    cell: (item) => h(UBadge, {
      label: item.row.original.verdict
        ? item.row.original.verdict
            .split('_')
            .map(part => part.charAt(0).toUpperCase() + part.slice(1))
            .join(' ')
        : 'N/A',
      color: item.row.original.verdict === 'auto_approved' || item.row.original.verdict === 'manual_approved'
        ? 'success'
        : item.row.original.verdict === 'auto_rejected' || item.row.original.verdict === 'manual_rejected'
          ? 'error'
          : 'neutral',
      variant: 'soft',
      size: 'sm',
    }),
  },
  {
    accessorKey: 'external_id',
    header: 'External ID',
    meta: { class: { th: 'min-w-32', td: 'min-w-32' } },
    cell: (item) => h('code', { class: 'text-xs text-muted' }, item.row.original.external_id || 'N/A'),
  },
  {
    id: 'filler',
    enableHiding: false,
    header: '',
    meta: { class: { th: 'w-full', td: 'w-full' } },
  },
];

const clearFilters = () => {
  selectedOptions.value = [];
  offset.value = 0;
};

const hasFilters = computed(() => selectedOptions.value.length > 0);
</script>

<template>
  <UDashboardPanel id="sessions">
    <template #body>
      <UiPageContainer title="Sessions" description="View verification records and their outcomes">
        <UiTable
          v-model:offset="offset"
          :columns="columns"
          :items="sessions"
          :status="status"
          pagination
          :total="total"
          :limit="limit"
        >
          <template #filters>
            <UCheckboxGroup
              v-model="selectedOptions"
              :options="optionLabels"
              name="session-options"
              class="flex gap-4"
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

        </UiTable>
      </UiPageContainer>
    </template>
  </UDashboardPanel>
</template>
