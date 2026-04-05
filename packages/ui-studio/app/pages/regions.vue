<script setup lang="ts">
import type { ContextMenuItem, TableColumn, TableRow } from "@nuxt/ui";
import { h } from "vue";

interface Region {
  id: string;
  slug: string;
  display_name: string;
  s3_provider_id: string | null;
  status: "active" | "inactive" | "maintenance";
  created_at: string;
  updated_at: string;
}

interface S3Provider {
  id: string;
  provider_type: "aws_s3" | "cloudflare_r2";
  endpoint_url: string;
  access_key_id: string;
  is_active: boolean;
}

const NuxtTime = resolveComponent("NuxtTime");
const toast = useToast();

const { data, status, refresh } = await useFetch<Region[]>("/api/infrastructure/regions");
const { data: providersData } = await useFetch<S3Provider[]>("/api/infrastructure/s3-providers?is_active=true");

const regions = computed(() => data.value ?? []);
const providers = computed(() => providersData.value ?? []);
const providerOptions = computed(() => [
  { label: "None", value: null as string | null },
  ...providers.value.map((provider) => {
    const maskedKey = provider.access_key_id.length > 6
      ? `${provider.access_key_id.slice(0, 4)}...${provider.access_key_id.slice(-2)}`
      : provider.access_key_id;
    return {
      label: `${provider.provider_type} - ${provider.endpoint_url} (${maskedKey})`,
      value: provider.id,
    };
  }),
]);
const providerNameById = computed(() => new Map(providers.value.map((provider) => [
  provider.id,
  `${provider.provider_type} - ${provider.endpoint_url}`,
])));
const regionStatusOptions = [
  { label: "Active", value: "active" },
  { label: "Inactive", value: "inactive" },
  { label: "Maintenance", value: "maintenance" },
];

const columns: TableColumn<Region>[] = [
  { accessorKey: "display_name", header: "Name" },
  { accessorKey: "slug", header: "Slug" },
  {
    accessorKey: "s3_provider_id",
    header: "S3 Provider",
    cell: (item) => {
      const value = item.row.original.s3_provider_id;
      return value ? (providerNameById.value.get(value) || value) : "None";
    },
  },
  { accessorKey: "status", header: "Status" },
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
const selected = ref<Region | null>(null);

const createForm = reactive({
  slug: "",
  display_name: "",
  s3_provider_id: null as string | null,
  status: "active" as Region["status"],
});

const editForm = reactive({
  slug: "",
  display_name: "",
  s3_provider_id: null as string | null,
  status: "active" as Region["status"],
});

const resetCreateForm = () => {
  createForm.slug = "";
  createForm.display_name = "";
  createForm.s3_provider_id = null;
  createForm.status = "active";
};

const openEdit = (row: TableRow<Region>) => {
  selected.value = row.original;
  editForm.slug = row.original.slug;
  editForm.display_name = row.original.display_name;
  editForm.s3_provider_id = row.original.s3_provider_id;
  editForm.status = row.original.status;
  editOpen.value = true;
};

const openDelete = (row: TableRow<Region>) => {
  selected.value = row.original;
  deleteOpen.value = true;
};

const contextItems = (row: TableRow<Region>): ContextMenuItem[] => [
  { type: "label", label: "Actions" },
  { label: "Edit", onSelect: () => openEdit(row) },
  { type: "separator" },
  { label: "Delete", color: "error", onSelect: () => openDelete(row) },
];

const createRegion = async () => {
  isSaving.value = true;
  try {
    await $fetch("/api/infrastructure/regions", {
      method: "POST",
      body: {
        slug: createForm.slug,
        display_name: createForm.display_name,
        s3_provider_id: createForm.s3_provider_id,
        status: createForm.status,
      },
    });
    createOpen.value = false;
    resetCreateForm();
    await refresh();
    toast.add({ title: "Region created", color: "success" });
  } catch {
    toast.add({ title: "Failed to create region", color: "error" });
  } finally {
    isSaving.value = false;
  }
};

const updateRegion = async () => {
  if (!selected.value) return;
  isSaving.value = true;
  try {
    await $fetch(`/api/infrastructure/regions/${selected.value.id}`, {
      method: "PATCH",
      body: {
        slug: editForm.slug,
        display_name: editForm.display_name,
        s3_provider_id: editForm.s3_provider_id,
        status: editForm.status,
      },
    });
    editOpen.value = false;
    await refresh();
    toast.add({ title: "Region updated", color: "success" });
  } catch {
    toast.add({ title: "Failed to update region", color: "error" });
  } finally {
    isSaving.value = false;
  }
};

const deleteRegion = async () => {
  if (!selected.value) return;
  isDeleting.value = true;
  try {
    await $fetch(`/api/infrastructure/regions/${selected.value.id}`, { method: "DELETE" });
    deleteOpen.value = false;
    await refresh();
    toast.add({ title: "Region deleted", color: "success" });
  } catch {
    toast.add({ title: "Failed to delete region", color: "error" });
  } finally {
    isDeleting.value = false;
  }
};
</script>

<template>
  <UDashboardPanel id="admin_regions">
    <template #body>
      <UiPageContainer title="Regions">
        <div class="flex justify-end">
          <UButton label="New Region" icon="i-heroicons-plus" @click="createOpen = true" />
        </div>

        <UiTable
          :columns="columns"
          :items="regions"
          :status="status ?? 'pending'"
          :get-context-menu-items="contextItems"
        />
      </UiPageContainer>

      <UModal v-model:open="createOpen" title="Create Region" description="Add a deployment region">
        <template #body>
          <UForm :state="createForm" class="space-y-4" @submit.prevent="createRegion">
            <UFormField label="Display Name" required>
              <UInput v-model="createForm.display_name" class="w-full" />
            </UFormField>
            <UFormField label="Slug" required>
              <UInput v-model="createForm.slug" class="w-full" />
            </UFormField>
            <UFormField label="Status">
              <USelect
                v-model="createForm.status"
                :items="regionStatusOptions"
                label-key="label"
                value-key="value"
                class="w-full"
              />
            </UFormField>
            <UFormField label="Backing S3 Provider (optional)">
              <USelect
                v-model="createForm.s3_provider_id"
                :items="providerOptions"
                label-key="label"
                value-key="value"
                class="w-full"
              />
            </UFormField>
            <div class="flex justify-end gap-2">
              <UButton variant="soft" @click="createOpen = false">Cancel</UButton>
              <UButton :loading="isSaving" type="submit">Create</UButton>
            </div>
          </UForm>
        </template>
      </UModal>

      <UModal v-model:open="editOpen" title="Edit Region" description="Update region details">
        <template #body>
          <div class="space-y-4">
            <UFormField label="Display Name" required>
              <UInput v-model="editForm.display_name" class="w-full" />
            </UFormField>
            <UFormField label="Slug" required>
              <UInput v-model="editForm.slug" class="w-full" />
            </UFormField>
            <UFormField label="Status">
              <USelect
                v-model="editForm.status"
                :items="regionStatusOptions"
                label-key="label"
                value-key="value"
                class="w-full"
              />
            </UFormField>
            <UFormField label="Backing S3 Provider (optional)">
              <USelect
                v-model="editForm.s3_provider_id"
                :items="providerOptions"
                label-key="label"
                value-key="value"
                class="w-full"
              />
            </UFormField>
            <div class="flex justify-end gap-2">
              <UButton variant="soft" @click="editOpen = false">Cancel</UButton>
              <UButton :loading="isSaving" @click="updateRegion">Save</UButton>
            </div>
          </div>
        </template>
      </UModal>

      <UModal v-model:open="deleteOpen" title="Delete Region" description="This action cannot be undone">
        <template #body>
          <div class="space-y-4">
            <p class="text-sm">Delete region <strong>{{ selected?.display_name }}</strong>?</p>
            <div class="flex justify-end gap-2">
              <UButton variant="soft" @click="deleteOpen = false">Cancel</UButton>
              <UButton color="error" :loading="isDeleting" @click="deleteRegion">Delete</UButton>
            </div>
          </div>
        </template>
      </UModal>
    </template>
  </UDashboardPanel>
</template>
