<script setup lang="ts">
import { h } from 'vue';
import type { TableColumn, TableRow } from '@nuxt/ui';
import * as z from 'zod';
import { ICONS } from '~/utils/icons'
import { MEMBER_PERMISSION_SCOPE_VALUES } from '@cplane/migrations/utils'

const store = useStore();

if (!store.organization?.id) {
  throw createError('Organization not found in store');
}

const toast = useToast();
const NuxtTime = resolveComponent("NuxtTime");
const limit = 10;
const offset = ref(0);
const search = ref('');

const { data, status, refresh } = await useFetch(
  () => `/api/organization/${store.organization?.id as ':organization_id'}/members`,
  {
    query: {
      limit,
      offset,
      search,
    },
  }
);

type Member = NonNullable<typeof data.value>['data'][number];

const addModalOpen = ref(false);
const isAdding = ref(false);
const deleteModalOpen = ref(false);
const isRemoving = ref(false);
const selectedMemberToDelete = ref<Member | null>(null);
const permissionsModalOpen = ref(false);
const isSavingPermissions = ref(false);
const selectedMemberForPermissions = ref<Member | null>(null);
const permissionsDraft = ref<Record<string, boolean>>({});

const members = computed(() => data.value?.data ?? []);
const total = computed(() => data.value?.pagination.total ?? 0);

const columns: TableColumn<Member>[] = [
  {
    accessorKey: 'user.name',
    header: 'Name',
    cell: (item) => item.row.original.user.name,
  },
  {
    accessorKey: 'user.email',
    header: 'Email',
    cell: (item) => item.row.original.user.email,
  },
  {
    accessorKey: 'role',
    header: 'Role',
    meta: { class: { td: 'capitalize' } },
    cell: (item) => item.row.original.role,
  },
  {
    accessorKey: 'created_at',
    header: 'Joined',
    cell: (item) => h('div', [
      h(NuxtTime, {
        datetime: item.row.original.created_at,
        locale: 'en',
        dateStyle: 'short',
        timeStyle: 'short',
        hour12: false
      })
    ]),
  },
];

const getContextMenuItems = (row: TableRow<Member>) => {
  const items: Array<Record<string, unknown>> = [
    {
      type: 'label' as const,
      label: 'Actions',
    },
  ];
  if (store.isOwner && row.original.role !== 'owner') {
    items.push({
      label: 'Edit Permissions',
      onSelect: () => {
        selectedMemberForPermissions.value = row.original;
        permissionsDraft.value = Object.fromEntries(
          MEMBER_PERMISSION_SCOPE_VALUES.map((scope) => [
            scope,
            row.original.permissions.includes(scope),
          ]),
        );
        permissionsModalOpen.value = true;
      },
    });
  }
  if (row.original.role !== 'owner') {
    items.push({
      label: 'Remove Member',
      color: 'error' as const,
      onSelect: () => {
        selectedMemberToDelete.value = row.original;
        deleteModalOpen.value = true;
      },
    });
  }
  return items;
};

const onSavePermissions = async () => {
  isSavingPermissions.value = true;
  try {
    if (!selectedMemberForPermissions.value) return;

    await $fetch(
      `/api/organization/${store.organization?.id as ':organization_id'}/members/${selectedMemberForPermissions.value.id as ':member_id'}/permissions`,
      {
        method: 'PUT',
        body: {
          permissions: Object.entries(permissionsDraft.value)
            .filter(([, enabled]) => enabled)
            .map(([scope]) => scope),
        },
      }
    );

    toast.add({
      title: 'Success',
      description: 'Permissions updated successfully',
      color: 'success',
    });

    permissionsModalOpen.value = false;
    selectedMemberForPermissions.value = null;
    refresh();
  } catch (error) {
    toast.add({
      title: 'An Error accured',
      description:
        error instanceof Error ? error.message : 'Failed to update permissions',
      color: 'error' as const,
    });
  } finally {
    isSavingPermissions.value = false;
  }
};

const onDeleteMember = async () => {
  isRemoving.value = true;
  try {
    if (!selectedMemberToDelete.value) return;

    await $fetch(
      `/api/organization/${store.organization?.id as ':organization_id'}/members/${selectedMemberToDelete.value.id as ':member_id'}`,
      {
        method: 'DELETE',
      }
    );

    toast.add({
      title: 'Success',
      description: 'Member removed successfully',
      color: 'success',
    });

    deleteModalOpen.value = false;
    selectedMemberToDelete.value = null;
    refresh();
  } catch (error) {
    toast.add({
      title: 'An Error accured',
      description:
        error instanceof Error ? error.message : 'Failed to remove member',
      color: 'error' as const,
    });
  } finally {
    isRemoving.value = false;
  }
};

const addMemberSchema = z.object({
  email: z.string().email('Invalid email address'),
});

type AddMemberSchema = z.output<typeof addMemberSchema>;

const addMemberState = reactive<Partial<AddMemberSchema>>({
  email: '',
});

const onAddMember = async () => {
  isAdding.value = true;
  try {
    await $fetch(`/api/organization/${store.organization?.id as ':organization_id'}/members`, {
      method: 'POST',
      body: {
        email: addMemberState.email,
      },
    });

    toast.add({
      title: 'Success',
      description: 'Member added successfully',
      color: 'success',
    });

    addModalOpen.value = false;
    addMemberState.email = '';
    refresh();
  } catch (error) {
    toast.add({
      title: 'An Error accured',
      description:
        error instanceof Error ? error.message : 'Failed to add member',
      color: 'error' as const,
    });
  } finally {
    isAdding.value = false;
  }
};
</script>

<template>
  <OrganizationSettingsPage title="Members">

    <section class="border-b border-dashed border-default pb-10">
      <div class="flex flex-col gap-5 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h2 class="text-xl font-normal tracking-[-0.02em]">Organization members</h2>
          <p class="mt-2 text-sm text-muted">Search and manage member access.</p>
        </div>
        <UButton :icon="ICONS.plus" color="primary" @click="addModalOpen = true">Add Member</UButton>
      </div>
      <div class="mt-8">
        <UiTable
          v-model:offset="offset"
          :status="status"
          :items="members"
          :columns="columns"
          :pagination="true"
          :total="total"
          :limit="limit"
          :get-context-menu-items="getContextMenuItems"
        >
          <template #filters>
            <UInput v-model="search" placeholder="Search members..." icon="i-heroicons:magnifying-glass" />
          </template>
        </UiTable>
      </div>
    </section>

    <UModal v-model:open="addModalOpen" title="Add Member" description="Invite a new member to your organization">
      <template #body>
        <UForm :schema="addMemberSchema" :state="addMemberState" class="space-y-4" @submit.prevent="onAddMember">
          <UFormField label="Email" name="email">
            <UInput v-model="addMemberState.email" type="email" placeholder="Enter member email" class="w-full" />
          </UFormField>
          <div class="flex justify-end gap-3 pt-2">
            <UButton type="button" variant="ghost" color="neutral" @click="addModalOpen = false">Cancel</UButton>
            <UButton :icon="ICONS.plus" color="primary" type="submit" :loading="isAdding">Add Member</UButton>
          </div>
        </UForm>
      </template>
    </UModal>

    <UModal v-model:open="deleteModalOpen" title="Remove Member" description="This action cannot be undone">
      <template #body>
        <div class="space-y-4">
          <p class="text-sm">
            Are you sure you want to remove <strong>{{ selectedMemberToDelete?.user.name }}</strong> ({{ selectedMemberToDelete?.user.email }}) from the organization?
          </p>
          <div class="flex justify-end gap-3 pt-2">
            <UButton variant="ghost" color="neutral" @click="deleteModalOpen = false">Cancel</UButton>
            <UButton :icon="ICONS.trash" color="error" :loading="isRemoving" @click="onDeleteMember">Remove</UButton>
          </div>
        </div>
      </template>
    </UModal>

    <UModal
      v-model:open="permissionsModalOpen"
      :title="`Permissions — ${selectedMemberForPermissions?.user.name ?? ''}`"
      description="Scopes mirror the API-key vocabulary and gate this member's access"
    >
      <template #body>
        <div class="max-h-96 space-y-1 overflow-y-auto pr-1">
          <UCheckbox
            v-for="scope in MEMBER_PERMISSION_SCOPE_VALUES"
            :key="scope"
            v-model="permissionsDraft[scope]"
            :label="scope"
          />
        </div>
        <div class="mt-4 flex justify-end gap-3">
          <UButton variant="ghost" color="neutral" @click="permissionsModalOpen = false">Cancel</UButton>
          <UButton color="primary" :loading="isSavingPermissions" @click="onSavePermissions">Save</UButton>
        </div>
      </template>
    </UModal>
  </OrganizationSettingsPage>
</template>
