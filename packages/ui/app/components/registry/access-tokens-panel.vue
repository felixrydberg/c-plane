<script setup lang="ts">
import { h } from 'vue'
import type { TableColumn } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

const props = defineProps<{ organizationId: string }>()
const route = useRoute()
const toast = useToast()
const endpoint = computed(() => `/api/cplane/organization/${props.organizationId as ':organization_id'}/registry/access-tokens` as const)
const { data: tokens, status, refresh } = await useFetch(endpoint, { default: () => [] })
const revokingId = ref('')
const refreshing = ref(false)
const search = ref('')
const UButton = resolveComponent('UButton')

const filteredTokens = computed(() => {
  const query = search.value.trim().toLowerCase()
  return query ? tokens.value.filter(token => token.name.toLowerCase().includes(query)) : tokens.value
})

async function revoke(token: NonNullable<typeof tokens.value>[number]) {
  revokingId.value = token.id
  try {
    await $fetch(`/api/cplane/organization/${props.organizationId as ':organization_id'}/registry/access-tokens/${token.id as ':token_id'}` as const, { method: 'DELETE' })
    await refresh()
  } catch {
    toast.add({ title: 'Could not revoke token', color: 'error' })
  } finally {
    revokingId.value = ''
  }
}

async function reloadTokens() {
  refreshing.value = true
  try {
    await refresh()
  } finally {
    refreshing.value = false
  }
}

const columns: TableColumn<NonNullable<typeof tokens.value>[number]>[] = [
  {
    accessorKey: 'name',
    header: 'Name',
    cell: ({ row }) => h('span', { class: 'truncate' }, row.original.name),
  },
  {
    accessorKey: 'created_at',
    header: 'Created',
    meta: { class: { th: 'hidden sm:table-cell', td: 'hidden sm:table-cell' } },
    cell: ({ row }) => new Date(row.original.created_at).toLocaleDateString(),
  },
  {
    id: 'actions',
    header: '',
    meta: { class: { th: 'text-right', td: 'text-right' } },
    cell: ({ row }) => h('div', { class: 'flex justify-end gap-2' }, [
      h(UButton, {
        icon: ICONS.pencil,
        color: 'neutral',
        size: 'sm',
        to: `/${route.params.organization_slug}/registry/access-tokens/${row.original.id}`,
      }, { default: () => 'Edit' }),
      h(UButton, {
        icon: ICONS.trash,
        color: 'error',
        size: 'sm',
        loading: revokingId.value === row.original.id,
        onClick: () => revoke(row.original),
      }, { default: () => 'Revoke' }),
    ]),
  },
]
</script>

<template>
  <section class="space-y-5">
    <div class="flex items-center gap-2">
      <UInput v-model="search" icon="i-heroicons:magnifying-glass" placeholder="Search access tokens..." aria-label="Search access tokens" class="min-w-0 flex-1" />
      <UButton :icon="ICONS.refresh" variant="ghost" color="neutral" :loading="refreshing" aria-label="Reload access tokens" @click="reloadTokens" />
    </div>
    <UiTable :status="status" :items="filteredTokens" :columns="columns" disable-header>
      <template #empty>
        <div class="flex flex-col items-center justify-center gap-3 py-14 text-center">
          <UIcon :name="ICONS.authentication" class="size-10 text-muted" />
          <p class="text-muted">{{ search ? 'No matching access tokens.' : 'No S2 - Registry access tokens for this organization.' }}</p>
        </div>
      </template>
    </UiTable>
  </section>
</template>
