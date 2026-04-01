<script setup lang="ts">
  import type { TableColumn } from '@nuxt/ui';
  import { upperFirst } from 'scule'
  import { refDebounced } from '@vueuse/core'

  const props = defineProps<{
    project?: string
  }>();

  const store = useStore();
  if (!store.organization) {
    throw createError({ statusCode: 400, statusMessage: 'No organization found in store' })
  }

  const UCheckbox = resolveComponent('UCheckbox');
  const query = ref('');
  const queryDebounced = refDebounced(query, 200);
  const offset = ref(0);
  const limit = ref(10);
  const selectedUsers = defineModel("selected", {
    type: Object as () => { [key: string]: boolean }, 
    default: () => ({})
  });

  const { data: users, status } = await useFetch(`/api/organization/${store.organization?.id as ':organization_id'}/members`, {
    method: "GET",
    query: {
      search: queryDebounced,
      offset: offset,
      limit: limit,
      excludeRequester: true,
      project_id: props.project || undefined
    },
  });

  type OrganizationMember = NonNullable<typeof users.value>['data'][number];
  const columns: TableColumn<OrganizationMember>[] = [
    {
      id: 'select',
      enableHiding: false,
      header: ({ table }) =>
        h(UCheckbox, {
          modelValue: table.getIsSomePageRowsSelected()
            ? 'indeterminate'
            : table.getIsAllPageRowsSelected(),
          'onUpdate:modelValue': (value: boolean | 'indeterminate') =>
            table.toggleAllPageRowsSelected(!!value),
          'aria-label': 'Select all'
        }),
      cell: ({ row }) =>
        h(UCheckbox, {
          modelValue: row.getIsSelected(),
          'onUpdate:modelValue': (value: boolean | 'indeterminate') => row.toggleSelected(!!value),
          'aria-label': 'Select row'
        })
    },
    {
      accessorKey: "name",
      header: "Name",
      cell: member => upperFirst(member.row.original.user.name),
    },
  ];
</script>

<template>
  <ui-search-table
    v-model:offset="offset"
    v-model:query="query"
    v-model:selected="selectedUsers"
    :items="users?.data"
    :loading="status === 'pending'"
    :columns="columns"
    :limit="10"
    :total="users?.pagination?.total || 0"
    :get-row-id="(row: OrganizationMember) => row.id"
  />
</template>
