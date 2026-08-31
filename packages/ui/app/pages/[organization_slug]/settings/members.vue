<script setup lang="ts">
import { h } from 'vue';
import type { TableColumn, TableRow } from '@nuxt/ui';
import * as z from 'zod';
import { ICONS } from '~/utils/icons'

const store = useStore();
const isOwner = computed(() => store.organization?.member?.role === 'owner');

if (!store.organization?.id) {
  throw createError('Organization not found in store');
}

const toast = useToast();
const NuxtTime = resolveComponent("NuxtTime");
const limit = 10;
const offset = ref(0);
const search = ref('');

const { data, status, refresh } = await useFetch(
  () => `/ui-api/organization/${store.organization?.id as ':organization_id'}/members`,
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
  if (!isOwner.value) return [];
  return [
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
};

const onDeleteMember = async () => {
  isRemoving.value = true;
  try {
    if (!selectedMemberToDelete.value) return;

    await $fetch(
      `/ui-api/organization/${store.organization?.id as ':organization_id'}/members/${selectedMemberToDelete.value.id as ':member_id'}`,
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
    await $fetch(`/ui-api/organization/${store.organization?.id as ':organization_id'}/invitations`, {
      method: 'POST',
      body: {
        email: addMemberState.email,
        role: 'member',
        organization_id: store.organization?.id,
      },
    });

    toast.add({
      title: 'Success',
      description: 'Invitation sent successfully',
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
          <p class="mt-2 text-sm text-muted">Search and manage member access.</p>
        </div>
        <UButton v-if="isOwner" :icon="ICONS.plus" color="primary" @click="addModalOpen = true">Add Member</UButton>
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
  </OrganizationSettingsPage>
</template>
