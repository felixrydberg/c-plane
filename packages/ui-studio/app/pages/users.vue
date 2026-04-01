<script setup lang="ts">
import type { TableColumn, ContextMenuItem, TableRow } from '@nuxt/ui';
import { useClipboard } from '@vueuse/core';
import { h } from 'vue';
import { upperFirst } from 'scule';

const limit = 20;
const offset = ref(0);
const NuxtTime = resolveComponent("NuxtTime");
const { copy } = useClipboard();
const toast = useToast();
const { data, status, error, refresh } = await useFetch('/api/admin/users', {
  query: {
    limit,
    offset
  }
});

type User = NonNullable<typeof data.value>['data'][number];

const viewDetailsModal = ref(false);
const banModal = ref(false);
const deleteModal = ref(false);
const selectedUser = ref<User | null>(null);
const selectedUserId = ref<string | undefined>();
const banReason = ref('');
const isBanning = ref(false);
const isDeleting = ref(false);

const columns: TableColumn<User>[] = [
  {
    accessorKey: 'name',
    header: 'Name',
  },
  {
    accessorKey: 'email',
    header: 'Email',
  },
  {
    accessorKey: 'role',
    header: 'Role',
  },
  {
    accessorKey: 'banned',
    header: 'Banned',
    cell: (item) => upperFirst(item.row.original.banned?.toString() || 'false'),
  },
  {
    accessorKey: 'createdAt',
    header: 'Created At',
    cell: (item) => h('div', [
      h(NuxtTime, {
        class: 'text-xs text-muted',
        datetime: item.row.original.createdAt,
        locale: 'en',
        month: 'numeric',
        day: 'numeric',
        year: '2-digit',
      })
    ]),
  },
];

const users = computed(() => data.value?.data ?? []);
const total = computed(() => data.value?.pagination.total ?? 0);

const openViewDetails = async (row: TableRow<User>) => {
  selectedUser.value = row.original;
  selectedUserId.value = row.original.id;
  viewDetailsModal.value = true;
};

const openBanModal = (row: TableRow<User>) => {
  selectedUser.value = row.original;
  banReason.value = '';
  banModal.value = true;
};

const handleBan = async () => {
  if (!selectedUser.value) return;
  
  isBanning.value = true;
  try {
    await $fetch(`/api/admin/users/${selectedUser.value.id}`, {
      method: 'POST',
      body: {
        reason: banReason.value,
      },
    });
    
    banModal.value = false;
    toast.add({
      title: 'Success',
      description: `User ${selectedUser.value.name} has been banned`,
      color: 'success'
    });
    await refresh();
  } catch (err) {
    console.error('Failed to ban user', err);
    toast.add({
      title: 'An Error accured',
      description: 'Failed to ban user',
      color: 'error'
    });
  } finally {
    isBanning.value = false;
  }
};

const openDeleteModal = (row: TableRow<User>) => {
  selectedUser.value = row.original;
  deleteModal.value = true;
};

const handleDelete = async () => {
  if (!selectedUser.value) return;
  
  isDeleting.value = true;
  try {
    await $fetch(`/api/admin/users/${selectedUser.value.id}`, {
      method: 'DELETE',
    });
    
    deleteModal.value = false;
    toast.add({
      title: 'Success',
      description: `User ${selectedUser.value.name} has been deleted`,
      color: 'success'
    });
    await refresh();
  } catch (err) {
    console.error('Failed to delete user', err);
    toast.add({
      title: 'An Error accured',
      description: 'Failed to delete user',
      color: 'error'
    });
  } finally {
    isDeleting.value = false;
  }
};

const getContextMenuItems = (row: TableRow<User>): ContextMenuItem[] => {
  return [
    {
      type: 'label' as const,
      label: 'Actions'
    },
    {
      label: 'View Details',
      onSelect: () => openViewDetails(row),
    },
    {
      label: 'Copy Email',
      onSelect: () => {
        copy(row.original.email);
        toast.add({
          title: 'Success',
          description: 'Email copied to clipboard',
          color: 'success'
        });
      },
    },
    {
      label: 'Copy User ID',
      onSelect: () => {
        copy(row.original.id);
        toast.add({
          title: 'Success',
          description: 'User ID copied to clipboard',
          color: 'success'
        });
      },
    },
    {
      type: 'separator' as const,
    },
    {
      label: 'Ban User',
      color: 'error',
      onSelect: () => openBanModal(row),
    },
    {
      label: 'Delete',
      color: 'error',
      onSelect: () => openDeleteModal(row),
    }
  ];
};
</script>

<template>
  <UDashboardPanel id="admin_users">
    <template #body>
      <UiPageContainer title="Users">
        <UiTable
          v-model:offset="offset"
          :columns="columns"
          :items="users"
          :status="status ?? 'pending'"
          :get-context-menu-items="getContextMenuItems"
          :pagination="true"
          :total="total"
          :limit="limit"
        />
      </UiPageContainer>
      <AdminUserDetailsModal
        v-if="viewDetailsModal"
        v-model="viewDetailsModal"
        :user="selectedUserId"
      />
    
      <UModal v-model:open="banModal" title="Ban User" description="Ban a user from the platform">
        <template #body>
          <div class="space-y-4">
            <p class="text-sm">Are you sure you want to ban <strong>{{ selectedUser?.name }}</strong>? This action cannot be undone.</p>
            <UFormField label="Ban Reason">
              <UTextarea
                v-model="banReason"
                class="w-full"
                placeholder="Enter reason for banning..."
              />
            </UFormField>
            <div class="flex gap-2 justify-end">
              <UButton
                variant="soft"
                @click="banModal = false"
              >
                Cancel
              </UButton>
              <UButton
                color="error"
                :loading="isBanning"
                @click="handleBan"
              >
                Ban User
              </UButton>
            </div>
          </div>
        </template>
      </UModal>

      <UModal v-model:open="deleteModal" title="Delete User" description="Permanently delete a user from the platform">
        <template #body>
          <div class="space-y-4">
            <p class="text-sm">Are you sure you want to delete <strong>{{ selectedUser?.name }}</strong>? This action cannot be undone.</p>
            <div class="flex gap-2 justify-end">
              <UButton
                variant="soft"
                @click="deleteModal = false"
              >
                Cancel
              </UButton>
              <UButton
                color="error"
                :loading="isDeleting"
                @click="handleDelete"
              >
                Delete User
              </UButton>
            </div>
          </div>
        </template>
      </UModal>
    </template>
  </UDashboardPanel>
</template>
