<script setup lang="ts">
import { h } from 'vue';
import type { TableColumn, TableRow } from '@nuxt/ui';
import * as z from 'zod';

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
  () => `/api/organization/${store.organization?.id as ':organization_id'}/api-keys`,
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
          h(UIcon, { name: 'i-heroicons:exclamation-triangle-solid', class: 'text-error' }),
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
    },
  },
  {
    type: 'separator' as const,
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
      `/api/organization/${store.organization?.id as ':organization_id'}/api-keys/${selectedKeyToDelete.value.id as ':api_key_id'}`,
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
      title: 'An Error occurred',
      description:
        error instanceof Error ? error.message : 'Failed to delete API key',
      color: 'error' as const,
    });
  }
};

const createApiKeySchema = z.object({
  name: z.string().min(1, 'Key name is required').max(255, 'Key name must be less than 255 characters'),
  allowed_ips: z.string().optional(),
  scopes: z.record(z.string(), z.boolean()).refine((scopes) => {
    return Object.values(scopes).some((enabled) => enabled);
  }, 'At least one scope must be selected'),
});

type CreateApiKeySchema = z.output<typeof createApiKeySchema>;

const createApiKeyState = reactive<Partial<CreateApiKeySchema>>({
  name: '',
  allowed_ips: '',
  scopes: {},
});

const expiryOptions = [
  { label: 'Never', value: 0 },
  { label: '1 Month', value: 1 },
  { label: '3 Months', value: 3 },
  { label: '6 Months', value: 6 },
  { label: '1 Year', value: 12 },
  { label: '2 Years', value: 24 },
];

const createSelectedExpiry = ref(3);

watch(detailsModalOpen, (isOpen) => {
  if (!isOpen) {
    createdKeyValue.value = undefined;
  }
});

const onCreateApiKeySubmit = async () => {
  try {
    const createdKey = await $fetch(`/api/organization/${store.organization?.id as ':organization_id'}/api-keys`, {
      method: 'POST',
      body: {
        name: createApiKeyState.name,
        scopes: createApiKeyState.scopes,
        expires_at: createSelectedExpiry.value,
        allowed_ips: createApiKeyState.allowed_ips || null,
      },
    });

    if (!createdKey || !createdKey.id) {
      throw new Error('Invalid response from server');
    }

    createModalOpen.value = false;
    createApiKeyState.name = '';
    createApiKeyState.allowed_ips = '';
    createApiKeyState.scopes = {};
    createSelectedExpiry.value = 3;
    
    selectedApiKeyId.value = createdKey.id;
    createdKeyValue.value = createdKey.key;
    detailsModalOpen.value = true;
    
    refresh();
  } catch (error) {
    toast.add({
      title: 'An Error occurred',
      description:
        error instanceof Error ? error.message : 'Failed to create API key',
      color: 'error' as const,
    });
  }
};
</script>

<template>
  <UDashboardPanel id="keys">
    <template #body>
      <UiPageContainer title="Authentication" size="max-w-xl">
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
          >
            <template #filters>
              <UButton leading-icon="i-heroicons:plus" variant="soft" @click="createModalOpen = true">
                Add Key
              </UButton>
            </template>
          </UiTable>
        </div>

        <UModal
          v-model:open="createModalOpen"
          title="Create API Key"
          description="Create a new API key for your organization"
        >
          <template #body>
            <UForm
              :schema="createApiKeySchema"
              :state="createApiKeyState"
              class="space-y-4"
              @submit.prevent="onCreateApiKeySubmit"
            >
              <UFormField label="Name" name="name">
                <UInput
                  v-model="createApiKeyState.name"
                  placeholder="Enter a name for the API key"
                  class="w-full"
                />
              </UFormField>
              <UFormField label="Expiration">
                <USelect
                  v-model="createSelectedExpiry"
                  :items="expiryOptions"
                  class="w-full"
                />
              </UFormField>
              <UFormField label="Allowed IPs" name="allowed_ips" description="Comma-separated list of allowed IPs. Leave empty to allow all.">
                <UInput
                  v-model="createApiKeyState.allowed_ips"
                  placeholder="e.g. 192.168.1.1, 10.0.0.1"
                  class="w-full"
                />
              </UFormField>
              <OrganizationApiKeyScopes v-model="createApiKeyState.scopes" />
              <div class="flex gap-2 justify-end">
                <UButton
                  type="button"
                  variant="soft"
                  @click="createModalOpen = false"
                >
                  Cancel
                </UButton>
                <UButton type="submit" color="primary">
                  Create
                </UButton>
              </div>
            </UForm>
          </template>
        </UModal>

        <OrganizationApiKeyDetailsModal
          v-if="store.organization && apiKeys.length > 0"
          v-model:open="detailsModalOpen"
          :api-key="createdKeyValue"
          :refresh="refresh"
          :api-key-id="selectedApiKeyId"
          :organization-id="store.organization?.id"
        />

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
