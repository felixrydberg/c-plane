<script setup lang="ts">
import type { ContextMenuItem, TableColumn, TableRow } from "@nuxt/ui";
import { h } from "vue";

interface Region {
  id: string;
  display_name: string;
}
interface Provider {
  id: string;
  provider_type: "aws_s3" | "cloudflare_r2";
  endpoint_url: string;
  provider_region: string | null;
  access_key_id: string;
  is_active: boolean;
  has_session_token: boolean;
  created_at: string;
  updated_at: string;
}

const NuxtTime = resolveComponent("NuxtTime");
const toast = useToast();

const { data, status, refresh } = await useFetch<Provider[]>("/api/infrastructure/s3-providers");

const providers = computed(() => data.value ?? []);

const providerTypeOptions = [
  { label: "AWS S3", value: "aws_s3" },
  { label: "Cloudflare R2", value: "cloudflare_r2" },
];

const columns: TableColumn<Provider>[] = [
  { accessorKey: "provider_type", header: "Provider" },
  { accessorKey: "endpoint_url", header: "Endpoint" },
  { accessorKey: "is_active", header: "Active" },
  {
    accessorKey: "updated_at",
    header: "Updated",
    cell: (item) =>
      h(NuxtTime, {
        class: "text-xs text-muted",
        datetime: item.row.original.updated_at,
        locale: "en",
        month: "numeric",
        day: "numeric",
        year: "2-digit",
      }),
  },
];

const createOpen = ref(false);
const editOpen = ref(false);
const deleteOpen = ref(false);
const isSaving = ref(false);
const isDeleting = ref(false);
const selected = ref<Provider | null>(null);

const createForm = reactive({
  provider_type: "aws_s3" as Provider["provider_type"],
  endpoint_url: "",
  provider_region: "",
  access_key_id: "",
  secret_access_key: "",
  session_token: "",
  is_active: true,
});

const editForm = reactive({
  provider_type: "aws_s3" as Provider["provider_type"],
  endpoint_url: "",
  provider_region: "",
  access_key_id: "",
  secret_access_key: "",
  session_token: "",
  clear_session_token: false,
  is_active: true,
});

const resetCreateForm = () => {
  createForm.provider_type = "aws_s3";
  createForm.endpoint_url = "";
  createForm.provider_region = "";
  createForm.access_key_id = "";
  createForm.secret_access_key = "";
  createForm.session_token = "";
  createForm.is_active = true;
};

const openEdit = (row: TableRow<Provider>) => {
  selected.value = row.original;
  editForm.provider_type = row.original.provider_type;
  editForm.endpoint_url = row.original.endpoint_url;
  editForm.provider_region = row.original.provider_region ?? "";
  editForm.access_key_id = row.original.access_key_id;
  editForm.secret_access_key = "";
  editForm.session_token = "";
  editForm.clear_session_token = false;
  editForm.is_active = row.original.is_active;
  editOpen.value = true;
};

const openDelete = (row: TableRow<Provider>) => {
  selected.value = row.original;
  deleteOpen.value = true;
};

const contextItems = (row: TableRow<Provider>): ContextMenuItem[] => [
  { type: "label", label: "Actions" },
  { label: "Edit", onSelect: () => openEdit(row) },
  { type: "separator" },
  { label: "Delete", color: "error", onSelect: () => openDelete(row) },
];

const createProvider = async () => {
  isSaving.value = true;
  try {
    await $fetch("/api/infrastructure/s3-providers", {
      method: "POST",
      body: {
        provider_type: createForm.provider_type,
        endpoint_url: createForm.endpoint_url,
        provider_region: createForm.provider_region || undefined,
        access_key_id: createForm.access_key_id,
        secret_access_key: createForm.secret_access_key,
        session_token: createForm.session_token || undefined,
        is_active: createForm.is_active,
      },
    });
    createOpen.value = false;
    resetCreateForm();
    await refresh();
    toast.add({ title: "S3 provider created", color: "success" });
  } catch {
    toast.add({ title: "Failed to create S3 provider", color: "error" });
  } finally {
    isSaving.value = false;
  }
};

const updateProvider = async () => {
  if (!selected.value) return;

  isSaving.value = true;
  try {
    await $fetch(`/api/infrastructure/s3-providers/${selected.value.id}`, {
      method: "PATCH",
      body: {
        provider_type: editForm.provider_type,
        endpoint_url: editForm.endpoint_url,
        provider_region: editForm.provider_region || null,
        access_key_id: editForm.access_key_id,
        secret_access_key: editForm.secret_access_key || undefined,
        session_token: editForm.clear_session_token
          ? null
          : editForm.session_token || undefined,
        is_active: editForm.is_active,
      },
    });
    editOpen.value = false;
    await refresh();
    toast.add({ title: "S3 provider updated", color: "success" });
  } catch {
    toast.add({ title: "Failed to update S3 provider", color: "error" });
  } finally {
    isSaving.value = false;
  }
};

const deleteProvider = async () => {
  if (!selected.value) return;

  isDeleting.value = true;
  try {
    await $fetch(`/api/infrastructure/s3-providers/${selected.value.id}`, {
      method: "DELETE",
    });
    deleteOpen.value = false;
    await refresh();
    toast.add({ title: "S3 provider deleted", color: "success" });
  } catch {
    toast.add({ title: "Failed to delete S3 provider", color: "error" });
  } finally {
    isDeleting.value = false;
  }
};
</script>

<template>
  <UDashboardPanel id="admin_s3_providers">
    <template #body>
      <UiPageContainer title="S3 Providers">
        <div class="flex justify-end">
          <UButton label="New Provider" icon="i-heroicons-plus" @click="createOpen = true" />
        </div>

        <UiTable
          :columns="columns"
          :items="providers"
          :status="status ?? 'pending'"
          :get-context-menu-items="contextItems"
        />
      </UiPageContainer>

      <UModal v-model:open="createOpen" title="Create S3 Provider" description="Configure region object storage backend">
        <template #body>
          <div class="space-y-4">
            <UFormField label="Provider Type" required>
              <USelect
                v-model="createForm.provider_type"
                :items="providerTypeOptions"
                label-key="label"
                value-key="value"
                class="w-full"
              />
            </UFormField>
            <UFormField label="Endpoint URL" required>
              <UInput v-model="createForm.endpoint_url" class="w-full" />
            </UFormField>
            <UFormField label="Provider Region">
              <UInput v-model="createForm.provider_region" class="w-full" />
            </UFormField>
            <UFormField label="Access Key ID" required>
              <UInput v-model="createForm.access_key_id" class="w-full" />
            </UFormField>
            <UFormField label="Secret Access Key" required>
              <UInput v-model="createForm.secret_access_key" type="password" class="w-full" />
            </UFormField>
            <UFormField label="Session Token">
              <UInput v-model="createForm.session_token" type="password" class="w-full" />
            </UFormField>
            <UFormField label="Active">
              <UCheckbox v-model="createForm.is_active" label="Enabled" />
            </UFormField>
            <div class="flex justify-end gap-2">
              <UButton variant="soft" @click="createOpen = false">Cancel</UButton>
              <UButton :loading="isSaving" @click="createProvider">Create</UButton>
            </div>
          </div>
        </template>
      </UModal>

      <UModal v-model:open="editOpen" title="Edit S3 Provider" description="Update region object storage backend">
        <template #body>
          <div class="space-y-4">
            <UFormField label="Provider Type" required>
              <USelect
                v-model="editForm.provider_type"
                :items="providerTypeOptions"
                label-key="label"
                value-key="value"
                class="w-full"
              />
            </UFormField>
            <UFormField label="Endpoint URL" required>
              <UInput v-model="editForm.endpoint_url" class="w-full" />
            </UFormField>
            <UFormField label="Provider Region">
              <UInput v-model="editForm.provider_region" class="w-full" />
            </UFormField>
            <UFormField label="Access Key ID" required>
              <UInput v-model="editForm.access_key_id" class="w-full" />
            </UFormField>
            <UFormField label="Secret Access Key (leave empty to keep current)">
              <UInput v-model="editForm.secret_access_key" type="password" class="w-full" />
            </UFormField>
            <UFormField label="Session Token (leave empty to keep current)">
              <UInput
                v-model="editForm.session_token"
                type="password"
                class="w-full"
                :disabled="editForm.clear_session_token"
              />
            </UFormField>
            <UFormField>
              <UCheckbox v-model="editForm.clear_session_token" label="Clear stored session token" />
            </UFormField>
            <UFormField label="Active">
              <UCheckbox v-model="editForm.is_active" label="Enabled" />
            </UFormField>
            <div class="flex justify-end gap-2">
              <UButton variant="soft" @click="editOpen = false">Cancel</UButton>
              <UButton :loading="isSaving" @click="updateProvider">Save</UButton>
            </div>
          </div>
        </template>
      </UModal>

      <UModal v-model:open="deleteOpen" title="Delete S3 Provider" description="This action cannot be undone">
        <template #body>
          <div class="space-y-4">
            <p class="text-sm">Delete S3 provider <strong>{{ selected?.endpoint_url }}</strong>?</p>
            <div class="flex justify-end gap-2">
              <UButton variant="soft" @click="deleteOpen = false">Cancel</UButton>
              <UButton color="error" :loading="isDeleting" @click="deleteProvider">Delete</UButton>
            </div>
          </div>
        </template>
      </UModal>
    </template>
  </UDashboardPanel>
</template>
