<script setup lang="ts">
import type { TableColumn, ContextMenuItem, TableRow } from '@nuxt/ui';
import { h } from 'vue';

const limit = 10;
const offset = ref(0);
const NuxtTime = resolveComponent("NuxtTime");

const { data, status, refresh } = await useFetch<{
  data: Array<{
    id: string;
    name: string;
    email: string;
    slug: string;
    logo: string | null;
    created_at: Date;
    polar_customer_id: string;
  }>;
  pagination: { total: number; limit: number; offset: number };
}>('/api/admin/organizations', {
  query: {
    limit,
    offset
  }
});

type Organization = NonNullable<typeof data.value>['data'][number];

const columns: TableColumn<Organization>[] = [
  {
    accessorKey: 'name',
    header: 'Name',
  },
  {
    accessorKey: 'email',
    header: 'Email',
  },
  {
    accessorKey: 'slug',
    header: 'Slug',
  },
  {
    accessorKey: 'created_at',
    header: 'Created At',
    cell: (item) => h('div', [
      h(NuxtTime, {
        class: 'text-xs text-muted',
        datetime: item.row.original.created_at,
        locale: 'en',
        month: 'numeric',
        day: 'numeric',
        year: '2-digit',
      })
    ]),
  },
];

const organizations = computed(() => data.value?.data ?? []);
const total = computed(() => data.value?.pagination.total ?? 0);

const detailsModalOpen = ref(false);
const editModalOpen = ref(false);
const membersModalOpen = ref(false);
const deleteModalOpen = ref(false);
const selectedOrganizationId = ref<string | undefined>();
const selectedOrganizationName = ref<string | undefined>();

const openDetailsModal = (row: TableRow<Organization>) => {
  selectedOrganizationId.value = row.original.id;
  detailsModalOpen.value = true;
};

const openEditModal = (row: TableRow<Organization>) => {
  selectedOrganizationId.value = row.original.id;
  selectedOrganizationName.value = row.original.name;
  editModalOpen.value = true;
};

const openMembersModal = (row: TableRow<Organization>) => {
  selectedOrganizationId.value = row.original.id;
  membersModalOpen.value = true;
};

const openDeleteModal = (row: TableRow<Organization>) => {
  selectedOrganizationId.value = row.original.id;
  deleteModalOpen.value = true;
};

const getActionItems = (row: TableRow<Organization>): ContextMenuItem[] => {
  return [
    {
      type: 'label' as const,
      label: 'Actions'
    },
    {
      label: 'View Details',
      onSelect: () => openDetailsModal(row),
    },
    {
      label: 'Edit',
      onSelect: () => openEditModal(row),
    },
    {
      label: 'View Members',
      onSelect: () => openMembersModal(row),
    },
    {
      type: 'separator' as const,
    },
    {
      label: 'Delete',
      color: 'error',
      onSelect: () => openDeleteModal(row),
    }
  ];
};

const onOrganizationUpdated = async () => {
  await refresh();
};

const onOrganizationDeleted = async () => {
  deleteModalOpen.value = false;
  selectedOrganizationId.value = undefined;
  await refresh();
};
</script>

<template>
  <UDashboardPanel id="admin_organizations">
    <template #body>
      <UiPageContainer title="Organizations">
        <UiTable
          v-model:offset="offset"
          :columns="columns"
          :items="organizations"
          :status="status ?? 'pending'"
          :get-context-menu-items="getActionItems"
          :pagination="true"
          :total="total"
          :limit="limit"
        />

        <AdminOrganizationDetailsModal
          v-model="detailsModalOpen"
          :organization-id="selectedOrganizationId"
        />

        <AdminOrganizationEditModal
          v-model="editModalOpen"
          :organization-id="selectedOrganizationId"
          :initial-name="selectedOrganizationName"
          @updated="onOrganizationUpdated"
        />

        <AdminOrganizationMembersModal
          v-model="membersModalOpen"
          :organization-id="selectedOrganizationId"
        />

        <AdminOrganizationDeleteModal
          v-model="deleteModalOpen"
          :organization-id="selectedOrganizationId"
          @deleted="onOrganizationDeleted"
        />
      </UiPageContainer>
    </template>
  </UDashboardPanel>
</template>
