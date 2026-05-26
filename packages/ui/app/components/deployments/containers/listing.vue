<script setup lang="ts">
import type { ContextMenuItem, TableColumn, TableRow } from '@nuxt/ui';
import type { ContainerRow, ContainerVersionRow } from '@cplane/ui-shared/types';

interface Container extends ContainerRow {
  current_version: ContainerVersionRow | null;
  project_id: string | null;
}

export interface ContainerWithProject extends Container {
  _projectName?: string;
  _projectId?: string;
}

const props = defineProps<{
  containers: ContainerWithProject[]
  organizationId: string
  projectId: string | null
  branchId: string | null
  status: string
}>();

const emit = defineEmits<{ refresh: [] }>();

const toast = useToast();
const editModalOpen = ref(false);
const selectedContainer = ref<ContainerWithProject | null>(null);

const columns: TableColumn<ContainerWithProject>[] = [
  {
    accessorKey: 'name',
    header: 'Name',
    meta: { class: { th: 'w-48 whitespace-nowrap', td: 'w-48 whitespace-nowrap' } },
    cell: (item) => item.row.original.name,
  },
  {
    accessorKey: '_projectName',
    header: 'Project',
    meta: { class: { th: 'w-36 whitespace-nowrap', td: 'w-36 whitespace-nowrap' } },
    cell: (item) => item.row.original._projectName,
  },
  {
    accessorKey: 'image',
    header: 'Image',
    meta: { class: { th: 'min-w-40', td: 'min-w-40' } },
    cell: (item) => h('code', { class: 'text-xs' }, item.row.original.current_version?.image ?? '—'),
  },
  {
    accessorKey: 'public',
    header: 'Visibility',
    meta: { class: { th: 'w-24 whitespace-nowrap', td: 'w-24 whitespace-nowrap' } },
    cell: (item) => {
      const isPublic = item.row.original.current_version?.public;
      return h(resolveComponent('UBadge'), {
        label: isPublic ? 'Public' : 'Private',
        color: isPublic ? 'success' : 'neutral',
        variant: 'soft',
        size: 'sm',
      });
    },
  },
  {
    accessorKey: 'port',
    header: 'Port',
    meta: { class: { th: 'w-16 whitespace-nowrap', td: 'w-16 whitespace-nowrap' } },
    cell: (item) => {
      const port = item.row.original.current_version?.port;
      return port ? String(port) : '—';
    },
  },
  {
    accessorKey: 'replicas',
    header: 'Replicas',
    meta: { class: { th: 'w-20 whitespace-nowrap', td: 'w-20 whitespace-nowrap' } },
    cell: (item) => String(item.row.original.current_version?.replica_count ?? 0),
  },
  {
    accessorKey: 'created_at',
    header: 'Created',
    meta: { class: { th: 'w-32 whitespace-nowrap', td: 'w-32 whitespace-nowrap' } },
    cell: (item) => h(resolveComponent('NuxtTime'), {
      class: 'text-xs text-muted',
      datetime: item.row.original.created_at,
      relative: true,
    }),
  },
];

const visibleColumns = computed(() => {
  if (props.projectId) {
    return columns.filter(c => c.accessorKey !== '_projectName');
  }
  return columns;
});

function openEdit(row: TableRow<ContainerWithProject>) {
  selectedContainer.value = row.original;
  editModalOpen.value = true;
}

async function deleteContainer(row: TableRow<ContainerWithProject>) {
  const container = row.original;
  if (!props.organizationId || !container.id) return;
  try {
    await $fetch(`/api/backend/organization/${props.organizationId}/containers/${container.id}?branch_id=${props.branchId ?? ''}`, {
      method: 'DELETE',
    });
    toast.add({ title: 'Container removed from branch', color: 'success' });
    emit('refresh');
  } catch {
    toast.add({ title: 'Failed to remove container', color: 'error' });
  }
}

const getContextMenuItems = (row: TableRow<ContainerWithProject>): ContextMenuItem[] => [
  { type: 'label' as const, label: 'Actions' },
  {
    label: 'Edit',
    icon: 'i-heroicons:pencil-square',
    onSelect: () => openEdit(row),
  },
  {
    label: 'Delete',
    icon: 'i-heroicons:trash',
    color: 'error' as const,
    onSelect: () => deleteContainer(row),
  },
];
</script>

<template>
  <div class="flex flex-col gap-6">
    <UiTable
      v-if="containers.length > 0"
      :columns="visibleColumns"
      :items="containers"
      :status="status"
      :get-context-menu-items="getContextMenuItems"
    >
      <template #empty>
        <div class="flex flex-col items-center gap-2 py-8">
          <UIcon :name="ICONS.containers" class="size-8 text-muted" />
          <p class="text-muted text-sm">No containers found.</p>
        </div>
      </template>
    </UiTable>

    <div
      v-else
      class="flex flex-col items-center justify-center py-16 gap-3 text-center border border-dashed border-default rounded-lg"
    >
      <UIcon :name="ICONS.containers" class="size-10 text-muted" />
      <p class="text-muted" v-if="projectId">No containers deployed yet. Deploy one to get started.</p>
      <p class="text-muted" v-else>Select a project to manage its containers.</p>
    </div>

    <DeploymentsContainersEditModal
      v-if="selectedContainer"
      v-model:open="editModalOpen"
      :organization-id="organizationId"
      :container-id="selectedContainer.id"
      :container-name="selectedContainer.name"
      :branch-id="branchId"
      :current-version="selectedContainer.current_version"
      @updated="emit('refresh')"
    />
  </div>
</template>
