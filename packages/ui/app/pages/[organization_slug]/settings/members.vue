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
const deleteModalOpen = ref(false);
const selectedMemberToDelete = ref<Member | null>(null);

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

const getContextMenuItems = (row: TableRow<Member>) => [
  {
    type: 'label' as const,
    label: 'Actions',
  },
  {
    label: 'Remove Member',
    color: 'error' as const,
    onSelect: () => {
      selectedMemberToDelete.value = row.original;
      deleteModalOpen.value = true;
    },
  },
];

const onDeleteMember = async () => {
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
  }
};
</script>

<template>
  <div class="flex flex-col gap-6 w-full mx-auto max-w-6xl">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-semibold">Members</h1>
        <p class="text-muted text-sm mt-1">Manage organization members.</p>
      </div>
      <UButton leading-icon="i-heroicons:plus" @click="addModalOpen = true">
        Add Member
      </UButton>
    </div>

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

    <UModal v-model:open="addModalOpen" title="Add Member" description="Invite a new member to your organization">
      <template #body>
        <UForm :schema="addMemberSchema" :state="addMemberState" class="space-y-4" @submit.prevent="onAddMember">
          <UFormField label="Email" name="email">
            <UInput v-model="addMemberState.email" type="email" placeholder="Enter member email" class="w-full" />
          </UFormField>
          <div class="flex justify-end gap-3 pt-2">
            <UButton type="button" variant="ghost" color="neutral" @click="addModalOpen = false">Cancel</UButton>
            <UButton type="submit">Add Member</UButton>
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
            <UButton color="error" @click="onDeleteMember">Remove</UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>
