<script setup lang="ts">
import { h } from 'vue';
import type { TableColumn, TableRow } from '@nuxt/ui';
import * as z from 'zod';
import { ICONS } from '~/utils/icons'

const store = useStore();

if (!store.organization?.id) {
  throw createError('Organization not found in store');
}

const toast = useToast();
const NuxtTime = resolveComponent("NuxtTime");
const UIcon = resolveComponent("UIcon");
const limit = 10;
const offset = ref(0);

const { data, status, refresh } = await useFetch(
  `/api/organization/${store.organization.id as ':organization_id'}/api-keys`,
  {
    query: {
      limit,
      offset,
    },
  }
);

type ApiKey = NonNullable<typeof data.value>['data'][number];

const detailsModalOpen = ref(false);
const selectedApiKeyId = ref<string>(data.value?.data[0]?.id || "");

const createModalOpen = ref(false);
const createdKeyValue = ref<string | undefined>();

const deleteModalOpen = ref(false);
const selectedKeyToDelete = ref<ApiKey | null>(null);
const apiKeys = computed(() => data.value?.data ?? []);
const total = computed(() => data.value?.pagination.total ?? 0);

const columns: TableColumn<ApiKey>[] = [
  {
    accessorKey: 'name',
    header: 'Name',
    cell: (item) => {
      const created_at = new Date(item.row.original.created_at);
      const expires_at = new Date(created_at);
      expires_at.setMonth(expires_at.getMonth() + (item.row.original.expires_at || 0));
      const isExpired = item.row.original.expires_at !== 0 && expires_at < new Date();
      if (isExpired && (item.row.original.expires_at || 0) > 0) {
        return h('div', { class: 'flex items-center gap-2' }, [
          h('span', item.row.original.name),
          h(UIcon, { name: 'i-heroicons:exclamation-triangle', class: 'text-error' }),
        ]);
      }
      return item.row.original.name;
    }
  },
  {
    accessorKey: 'expires_at',
    header: 'Expires',
    cell: (item) => {
      const months = item.row.original.expires_at;
      if (!months) return 'Never';
      const expiryDate = new Date(item.row.original.created_at);
      expiryDate.setMonth(expiryDate.getMonth() + months);
      return h('div', { class: 'flex items-center gap-2' }, [
        h(NuxtTime, {
          class: 'text-sm text-muted',
          datetime: expiryDate.toISOString(),
          locale: 'en',
          dateStyle: 'short',
        }),
      ]);
    },
  },
  {
    accessorKey: 'created_at',
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
    label: 'View Details',
    onSelect: () => {
      selectedApiKeyId.value = row.original.id;
      detailsModalOpen.value = true;
    }
  },
  {
    label: 'Delete',
    color: 'error' as const,
    onSelect: () => {
      selectedKeyToDelete.value = row.original;
      deleteModalOpen.value = true;
    }
  }
];

const onDeleteKey = async () => {
  if (!selectedKeyToDelete.value) return;

  try {
    await $fetch(`/api/organization/${store.organization?.id as ':organization_id'}/api-keys/${selectedKeyToDelete.value.id as ':api_key_id'}`, {
      method: 'DELETE'
    });

    toast.add({
      title: 'Success',
      description: 'API key deleted successfully.',
      color: 'success',
    });
    deleteModalOpen.value = false;
    selectedKeyToDelete.value = null;
    refresh();
  } catch {
    toast.add({
      title: 'Error',
      description: 'Failed to delete API key.',
      color: 'error',
    });
  }
};

const createSchema = z.object({
  name: z.string().min(1, 'Name is required'),
  expires_at: z.number().min(0).default(0),
});

type CreateSchema = z.output<typeof createSchema>;
const createState = reactive<CreateSchema>({
  name: '',
  expires_at: 0,
});
const createScopes = ref<Record<string, boolean>>({});

const onCreateKey = async () => {
  if (!Object.values(createScopes.value).some(Boolean)) {
    toast.add({ title: 'Select at least one scope', color: 'error' });
    return;
  }
  try {
    const key = await $fetch<{ id: string; key: string }>(`/api/organization/${store.organization?.id as ':organization_id'}/api-keys`, {
      method: 'POST',
      body: { name: createState.name, expires_at: createState.expires_at, scopes: createScopes.value }
    });
    createdKeyValue.value = key.key;
    createModalOpen.value = false;
    createState.name = '';
    createState.expires_at = 0;
    createScopes.value = {};
    refresh();
  } catch {
    toast.add({
      title: 'Error',
      description: 'Failed to create API key.',
      color: 'error',
    });
  }
};
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-semibold">Authentication</h1>
        <p class="text-muted text-sm mt-1">Manage API keys for your organization.</p>
      </div>
      <UButton :icon="ICONS.plus" color="primary" @click="createModalOpen = true">
        Create API Key
      </UButton>
    </div>

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

    <UModal v-model:open="createModalOpen" title="Create API Key" description="Create a new API key for programmatic access">
      <template #body>
        <UForm :schema="createSchema" :state="createState" class="space-y-4" @submit.prevent="onCreateKey">
          <UFormField label="Name" name="name" required>
            <UInput v-model="createState.name" placeholder="My API Key" class="w-full" />
          </UFormField>
          <UFormField label="Expiration" name="expires_at" description="Expiration in months (0 = never)">
            <UInput v-model="createState.expires_at" type="number" min="0" class="w-full" />
          </UFormField>
          <OrganizationApiKeyScopes v-model="createScopes" />
          <div class="flex justify-end gap-3 pt-2">
            <UButton variant="ghost" color="neutral" type="button" @click="createModalOpen = false">Cancel</UButton>
            <UButton :icon="ICONS.plus" color="primary" type="submit">Create</UButton>
          </div>
        </UForm>
      </template>
    </UModal>

    <UModal v-model:open="detailsModalOpen" title="API Key Details" description="View API key configuration">
      <template #body>
        <OrganizationApiKeyDetailsModal
          :api-key-id="selectedApiKeyId"
          @close="detailsModalOpen = false"
        />
      </template>
    </UModal>

    <UModal v-model:open="deleteModalOpen" title="Delete API Key" description="This action cannot be undone">
      <template #body>
        <div class="space-y-4">
          <p class="text-sm">Are you sure you want to delete <strong>{{ selectedKeyToDelete?.name }}</strong>?</p>
          <div class="flex justify-end gap-3 pt-2">
            <UButton variant="ghost" color="neutral" @click="deleteModalOpen = false">Cancel</UButton>
            <UButton :icon="ICONS.trash" color="error" @click="onDeleteKey">Delete</UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>
