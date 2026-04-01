<script setup lang="ts">
import { h } from 'vue';
import type { TableColumn, TableRow } from '@nuxt/ui';

const toast = useToast();
const NuxtTime = resolveComponent("NuxtTime");
const limit = 10;
const offset = ref(0);

const { data, status, refresh } = await useFetch(
  () => `/api/admin/api-keys`,
  {
    query: {
      limit,
      offset,
    },
  }
);

type ApiKey = NonNullable<typeof data.value>['data'][number];

const deleteModalOpen = ref(false);
const selectedKeyToDelete = ref<ApiKey | null>(null);
const apiKeys = computed(() => data.value?.data ?? []);
const total = computed(() => data.value?.pagination.total ?? 0);

const columns: TableColumn<ApiKey>[] = [
  {
    accessorKey: 'name',
    header: 'Name',
  },
  {
    accessorKey: 'organization slug',
    header: 'Organization Slug',
  },
  {
    accessorKey: 'created at',
    header: 'Created At',
    cell: (item) => h('div', [
      h(NuxtTime, {
        class: 'text-xs text-muted',
        datetime: item.row.original.created_at,
        locale: 'en',
        dateStyle: 'short',
        timeStyle: 'short',
        hour12: false
      })
    ]),
  },
];

const getContextMenuItems = (row: TableRow<ApiKey>) => [
  {
    type: 'label' as const,
    label: 'Actions'
  },
  {
    label: 'Delete',
    color: 'error' as const,
    onSelect: () => {
      selectedKeyToDelete.value = row.original;
      deleteModalOpen.value = true;
    },
  },
];

const onDeleteApiKey = async () => {
  try {
    if (!selectedKeyToDelete.value) return;

    await $fetch(
      `/api/admin/api-keys/${selectedKeyToDelete.value.id}`,
      {
        method: 'DELETE',
      }
    );

    toast.add({
      title: 'Success',
      description: 'API key deleted successfully',
      color: 'success',
    });

    deleteModalOpen.value = false;
    selectedKeyToDelete.value = null;
    refresh();
  } catch (error) {
    toast.add({
      title: 'An Error accured',
      description:
        error instanceof Error ? error.message : 'Failed to delete API key',
      color: 'error' as const,
    });
  }
};
</script>

<template>
  <UDashboardPanel id="api-keys">
    <template #body>
      <UiPageContainer title="API Keys" size="max-w-6xl">
        <div class="space-y-4">
          <UiTable
            v-model:offset="offset"
            :status="status"
            :items="apiKeys"
            :columns="columns"
            :pagination="true"
            :total="total"
            :limit="limit"
            :get-context-menu-items="getContextMenuItems"
          />
        </div>

        <UModal
          v-model:open="deleteModalOpen"
          title="Delete API Key"
          description="This action cannot be undone"
        >
          <template #body>
            <div class="space-y-4">
              <p class="text-sm">
                Are you sure you want to delete <strong>{{ selectedKeyToDelete?.name }}</strong>? This action cannot be undone.
              </p>
              <div class="flex gap-2 justify-end">
                <UButton
                  variant="soft"
                  @click="deleteModalOpen = false"
                >
                  Cancel
                </UButton>
                <UButton
                  color="error"
                  @click="onDeleteApiKey"
                >
                  Delete
                </UButton>
              </div>
            </div>
          </template>
        </UModal>
      </UiPageContainer>
    </template>
  </UDashboardPanel>
</template>
