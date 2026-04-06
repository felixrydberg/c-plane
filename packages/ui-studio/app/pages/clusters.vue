<script setup lang="ts">
import type { ContextMenuItem, FormSubmitEvent, TableColumn, TableRow } from "@nuxt/ui";
import { h } from "vue";
import * as z from "zod";

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

const createSchema = z.object({
  region_id: z.string().uuid("Please select a region"),
  name: z.string().trim().min(1, "Name is required"),
  slug: z.string().trim().min(1, "Slug is required"),
});

type CreateSchema = z.output<typeof createSchema>;

const createForm = reactive<CreateSchema>({
  region_id: "",
  slug: "",
  name: "",
});

interface JoinCredential {
  id: string;
  cluster_id: string;
  token: string;
  expires_at: string;
}

const joinCredential = ref<JoinCredential | null>(null);

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
};

const closeCreateModal = () => {
  createOpen.value = false;
  joinCredential.value = null;
  resetCreateForm();
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

const createCluster = async (event: FormSubmitEvent<CreateSchema>) => {
  isSaving.value = true;
  try {
    const result = await $fetch<{ cluster: Cluster; join_credential: JoinCredential }>("/api/infrastructure/clusters", {
      method: "POST",
      body: event.data,
    });
    joinCredential.value = result.join_credential;
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

      <UModal v-model:open="createOpen" :title="joinCredential ? 'Cluster Created' : 'Create Cluster'" :description="joinCredential ? 'Save the join token — it will not be shown again.' : 'Register an execution cluster'">
        <template #body>
          <UForm v-if="!joinCredential" :schema="createSchema" :state="createForm" class="space-y-4" @submit="createCluster">
            <UFormField label="Region" name="region_id" required>
              <USelect
                v-model="createForm.region_id"
                :items="regionOptions"
                label-key="label"
                value-key="value"
                placeholder="Select a region"
                class="w-full"
              />
            </UFormField>
            <UFormField label="Name" name="name" required><UInput v-model="createForm.name" class="w-full" /></UFormField>
            <UFormField label="Slug" name="slug" required><UInput v-model="createForm.slug" class="w-full" /></UFormField>
            <div class="flex justify-end gap-2">
              <UButton variant="soft" @click="closeCreateModal">Cancel</UButton>
              <UButton type="submit" :loading="isSaving">Create</UButton>
            </div>
          </UForm>
          <div v-else class="space-y-4">
            <p class="text-sm text-muted">Pass this token to the cluster agent. It expires at {{ new Date(joinCredential.expires_at).toLocaleString() }} and can only be used once.</p>
            <UFormField label="Join Token">
              <UInput :model-value="joinCredential.token" readonly class="w-full font-mono" />
            </UFormField>
            <div class="flex justify-end">
              <UButton @click="closeCreateModal">Done</UButton>
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
