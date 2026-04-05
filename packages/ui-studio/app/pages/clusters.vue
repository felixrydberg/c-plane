<script setup lang="ts">
import type { ContextMenuItem, TableColumn, TableRow } from "@nuxt/ui";
import { h } from "vue";

interface Region {
  id: string;
  display_name: string;
}

interface Cluster {
  id: string;
  region_id: string;
  slug: string;
  name: string;
  agent_id: string;
  agent_endpoint: string;
  status: "pending" | "bootstrapping" | "healthy" | "draining" | "offline" | "removed";
  capacity_allocatable: number;
  capacity_used: number;
  health_status: "healthy" | "degraded" | "offline";
  updated_at: string;
}

const NuxtTime = resolveComponent("NuxtTime");
const toast = useToast();

const { data, status, refresh } = await useFetch<Cluster[]>("/api/infrastructure/clusters");
const { data: regionsData } = await useFetch<Region[]>("/api/infrastructure/regions");

const regions = computed(() => regionsData.value ?? []);
const clusters = computed(() => data.value ?? []);
const regionNameById = computed(() => new Map(regions.value.map((r) => [r.id, r.display_name])));
const regionOptions = computed(() => regions.value.map((region) => ({
  label: region.display_name,
  value: region.id,
})));

const columns: TableColumn<Cluster>[] = [
  { accessorKey: "name", header: "Name" },
  { accessorKey: "slug", header: "Slug" },
  {
    accessorKey: "region_id",
    header: "Region",
    cell: (item) => regionNameById.value.get(item.row.original.region_id) || item.row.original.region_id,
  },
  { accessorKey: "status", header: "Status" },
  { accessorKey: "health_status", header: "Health" },
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
const selected = ref<Cluster | null>(null);

const createForm = reactive({
  region_id: "",
  slug: "",
  name: "",
  agent_id: "",
  agent_endpoint: "",
  status: "pending" as Cluster["status"],
  capacity_allocatable: 0,
  capacity_used: 0,
  health_status: "healthy" as Cluster["health_status"],
});

const editForm = reactive({
  region_id: "",
  slug: "",
  name: "",
  agent_id: "",
  agent_endpoint: "",
  status: "pending" as Cluster["status"],
  capacity_allocatable: 0,
  capacity_used: 0,
  health_status: "healthy" as Cluster["health_status"],
});

const resetCreateForm = () => {
  createForm.region_id = "";
  createForm.slug = "";
  createForm.name = "";
  createForm.agent_id = "";
  createForm.agent_endpoint = "";
  createForm.status = "pending";
  createForm.capacity_allocatable = 0;
  createForm.capacity_used = 0;
  createForm.health_status = "healthy";
};

const openEdit = (row: TableRow<Cluster>) => {
  selected.value = row.original;
  editForm.region_id = row.original.region_id;
  editForm.slug = row.original.slug;
  editForm.name = row.original.name;
  editForm.agent_id = row.original.agent_id;
  editForm.agent_endpoint = row.original.agent_endpoint;
  editForm.status = row.original.status;
  editForm.capacity_allocatable = row.original.capacity_allocatable;
  editForm.capacity_used = row.original.capacity_used;
  editForm.health_status = row.original.health_status;
  editOpen.value = true;
};

const openDelete = (row: TableRow<Cluster>) => {
  selected.value = row.original;
  deleteOpen.value = true;
};

const contextItems = (row: TableRow<Cluster>): ContextMenuItem[] => [
  { type: "label", label: "Actions" },
  { label: "Edit", onSelect: () => openEdit(row) },
  { type: "separator" },
  { label: "Delete", color: "error", onSelect: () => openDelete(row) },
];

const createCluster = async () => {
  isSaving.value = true;
  try {
    await $fetch("/api/infrastructure/clusters", {
      method: "POST",
      body: { ...createForm },
    });
    createOpen.value = false;
    resetCreateForm();
    await refresh();
    toast.add({ title: "Cluster created", color: "success" });
  } catch {
    toast.add({ title: "Failed to create cluster", color: "error" });
  } finally {
    isSaving.value = false;
  }
};

const updateCluster = async () => {
  if (!selected.value) return;
  isSaving.value = true;
  try {
    await $fetch(`/api/infrastructure/clusters/${selected.value.id}`, {
      method: "PATCH",
      body: { ...editForm },
    });
    editOpen.value = false;
    await refresh();
    toast.add({ title: "Cluster updated", color: "success" });
  } catch {
    toast.add({ title: "Failed to update cluster", color: "error" });
  } finally {
    isSaving.value = false;
  }
};

const deleteCluster = async () => {
  if (!selected.value) return;
  isDeleting.value = true;
  try {
    await $fetch(`/api/infrastructure/clusters/${selected.value.id}`, { method: "DELETE" });
    deleteOpen.value = false;
    await refresh();
    toast.add({ title: "Cluster deleted", color: "success" });
  } catch {
    toast.add({ title: "Failed to delete cluster", color: "error" });
  } finally {
    isDeleting.value = false;
  }
};
</script>

<template>
  <UDashboardPanel id="admin_clusters">
    <template #body>
      <UiPageContainer title="Clusters">
        <div class="flex justify-end">
          <UButton label="New Cluster" icon="i-heroicons-plus" @click="createOpen = true" />
        </div>

        <UiTable
          :columns="columns"
          :items="clusters"
          :status="status ?? 'pending'"
          :get-context-menu-items="contextItems"
        />
      </UiPageContainer>

      <UModal v-model:open="createOpen" title="Create Cluster" description="Register an execution cluster">
        <template #body>
          <div class="space-y-4">
            <UFormField label="Region" required>
              <USelect
                v-model="createForm.region_id"
                :items="regionOptions"
                label-key="label"
                value-key="value"
                placeholder="Select a region"
                class="w-full"
              />
            </UFormField>
            <UFormField label="Name" required><UInput v-model="createForm.name" class="w-full" /></UFormField>
            <UFormField label="Slug" required><UInput v-model="createForm.slug" class="w-full" /></UFormField>
            <UFormField label="Agent ID" required><UInput v-model="createForm.agent_id" class="w-full" /></UFormField>
            <UFormField label="Agent Endpoint" required><UInput v-model="createForm.agent_endpoint" class="w-full" /></UFormField>
            <UFormField label="Status"><UInput v-model="createForm.status" class="w-full" /></UFormField>
            <UFormField label="Health Status"><UInput v-model="createForm.health_status" class="w-full" /></UFormField>
            <UFormField label="Capacity Allocatable"><UInput v-model="createForm.capacity_allocatable" type="number" class="w-full" /></UFormField>
            <UFormField label="Capacity Used"><UInput v-model="createForm.capacity_used" type="number" class="w-full" /></UFormField>
            <div class="flex justify-end gap-2">
              <UButton variant="soft" @click="createOpen = false">Cancel</UButton>
              <UButton :loading="isSaving" @click="createCluster">Create</UButton>
            </div>
          </div>
        </template>
      </UModal>

      <UModal v-model:open="editOpen" title="Edit Cluster" description="Update cluster details">
        <template #body>
          <div class="space-y-4">
            <UFormField label="Region" required>
              <USelect
                v-model="editForm.region_id"
                :items="regionOptions"
                label-key="label"
                value-key="value"
                placeholder="Select a region"
                class="w-full"
              />
            </UFormField>
            <UFormField label="Name" required><UInput v-model="editForm.name" class="w-full" /></UFormField>
            <UFormField label="Slug" required><UInput v-model="editForm.slug" class="w-full" /></UFormField>
            <UFormField label="Agent ID" required><UInput v-model="editForm.agent_id" class="w-full" /></UFormField>
            <UFormField label="Agent Endpoint" required><UInput v-model="editForm.agent_endpoint" class="w-full" /></UFormField>
            <UFormField label="Status"><UInput v-model="editForm.status" class="w-full" /></UFormField>
            <UFormField label="Health Status"><UInput v-model="editForm.health_status" class="w-full" /></UFormField>
            <UFormField label="Capacity Allocatable"><UInput v-model="editForm.capacity_allocatable" type="number" class="w-full" /></UFormField>
            <UFormField label="Capacity Used"><UInput v-model="editForm.capacity_used" type="number" class="w-full" /></UFormField>
            <div class="flex justify-end gap-2">
              <UButton variant="soft" @click="editOpen = false">Cancel</UButton>
              <UButton :loading="isSaving" @click="updateCluster">Save</UButton>
            </div>
          </div>
        </template>
      </UModal>

      <UModal v-model:open="deleteOpen" title="Delete Cluster" description="This action cannot be undone">
        <template #body>
          <div class="space-y-4">
            <p class="text-sm">Delete cluster <strong>{{ selected?.name }}</strong>?</p>
            <div class="flex justify-end gap-2">
              <UButton variant="soft" @click="deleteOpen = false">Cancel</UButton>
              <UButton color="error" :loading="isDeleting" @click="deleteCluster">Delete</UButton>
            </div>
          </div>
        </template>
      </UModal>
    </template>
  </UDashboardPanel>
</template>
