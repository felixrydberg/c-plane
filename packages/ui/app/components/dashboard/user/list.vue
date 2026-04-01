<script setup lang="ts">

const props = defineProps({
  users: {
    type: Array as PropType<Array<User>>,
    required: true,
  },
  status: {
    type: String,
    required: true,
  },
})

const columns = [
  {
    accessorKey: 'name',
    header: 'Name',
    cell: ({ row }: { row: { original: User } }) => {
      return row.original.name;
    }
  },
  {
    accessorKey: 'email',
    header: 'Email',
    cell: ({ row }: { row: { original: User } }) => {
      return row.original.email;
    }
  },
]
</script>

<template>
  <ui-list-table :items="users" :status="status" :columns="columns"/>
</template>
