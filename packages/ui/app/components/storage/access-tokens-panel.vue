<script setup lang="ts">
import { h } from 'vue'
import type { TableColumn } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

const props = defineProps<{ organizationId: string; projectId: string }>()
const route = useRoute()
const toast = useToast()
const isOwner = computed(() => useStore().organization?.member?.role === 'owner')
const endpoint = computed(() => `/api/organization/${props.organizationId as ':organization_id'}/projects/${props.projectId as ':project_id'}/storage/access-tokens` as const)
const { data: tokens, status, refresh } = await useCplaneFetch(endpoint, { default: () => [] })
const revokingId = ref('')
const refreshing = ref(false)
const search = ref('')
const UButton = resolveComponent('UButton')

const filteredTokens = computed(() => {
  const query = search.value.trim().toLowerCase()
  return query ? tokens.value.filter(token => token.name.toLowerCase().includes(query) || token.access_key_id.toLowerCase().includes(query)) : tokens.value
})

async function revoke(token: NonNullable<typeof tokens.value>[number]) {
  revokingId.value = token.id
  try {
    await cplaneFetch(`/api/organization/${props.organizationId as ':organization_id'}/projects/${props.projectId as ':project_id'}/storage/access-tokens/${token.id as ':token_id'}` as const, { method: 'DELETE' })
    await refresh()
  } catch { toast.add({ title: 'Could not revoke token', color: 'error' }) }
  finally { revokingId.value = '' }
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
    accessorKey: 'access_key_id',
    header: 'Access key ID',
    meta: { class: { th: 'hidden sm:table-cell', td: 'hidden sm:table-cell' } },
    cell: ({ row }) => h('span', { class: 'font-mono text-xs' }, row.original.access_key_id),
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
    cell: ({ row }) => isOwner.value ? h('div', { class: 'flex justify-end gap-2' }, [
      h(UButton, {
        icon: ICONS.pencil,
        color: 'neutral',
        size: 'sm',
        to: `/${route.params.organization_slug}/storage/${props.projectId}/access-tokens/${row.original.id}`,
      }, { default: () => 'Edit' }),
      h(UButton, {
        icon: ICONS.trash,
        color: 'error',
        size: 'sm',
        loading: revokingId.value === row.original.id,
        onClick: () => revoke(row.original),
      }, { default: () => 'Revoke' }),
    ]) : null,
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
          <p class="text-muted">{{ search ? 'No matching access tokens.' : 'No Object Storage access tokens for this project.' }}</p>
        </div>
      </template>
    </UiTable>
  </section>
</template>
