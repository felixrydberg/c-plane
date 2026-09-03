<script setup lang="ts">
import { h } from 'vue'
import type { Container } from '@cplane/sdk'
import type { TableColumn } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

type ContainerWithProject = Container & {
  _projectName?: string
  _projectId?: string
}

const props = defineProps<{
  containers: ContainerWithProject[]
  organizationId: string
  projectId: string | null
  environmentId: string | null
  revisionId?: string
  draftRevisionId?: string
  canRemove: boolean
  status: string
  hasError?: boolean
}>()

const emit = defineEmits<{ refresh: [view: 'draft' | 'deployed'] }>()

const toast = useToast()
const route = useRoute()
const deleteTarget = ref<ContainerWithProject | null>(null)
const removeAsDraft = ref(false)
const removing = ref(false)
const UButton = resolveComponent('UButton')
const UDropdownMenu = resolveComponent('UDropdownMenu')
const NuxtLink = resolveComponent('NuxtLink')
const NuxtTime = resolveComponent('NuxtTime')

async function confirmDelete() {
  if (!deleteTarget.value || !props.organizationId || !props.draftRevisionId || !deleteTarget.value.id) return
  const deploy = !removeAsDraft.value
  removing.value = true
  try {
    await cplaneFetch(`/api/organization/${props.organizationId as ':organization_id'}/containers/${deleteTarget.value.id as ':container_id'}` as const, {
      method: 'DELETE',
      query: { environment_id: props.environmentId ?? undefined, timeline_id: props.draftRevisionId, deploy: deploy || undefined },
    })
    toast.add({ title: deploy ? 'Container removed and deployed' : 'Container removed from draft', color: 'success' })
    deleteTarget.value = null
    emit('refresh', deploy ? 'deployed' : 'draft')
  } catch {
    toast.add({ title: deploy ? 'Failed to remove and deploy container' : 'Failed to remove container from draft', color: 'error' })
  } finally {
    removing.value = false
  }
}

function containerUrl(containerId: string) {
  return {
    path: `/${route.params.organization_slug}/containers/${props.projectId}/${props.environmentId}/${containerId}`,
    query: props.revisionId ? { revision: props.revisionId } : undefined,
  }
}

const columns: TableColumn<ContainerWithProject>[] = [
  {
    accessorKey: 'name',
    header: 'Service',
    cell: ({ row }) => h(NuxtLink, {
      to: containerUrl(row.original.id),
      class: 'flex min-w-0 items-center gap-2',
    }, [
      h('span', { class: 'truncate font-medium text-primary group-hover:underline group-hover:underline-offset-4' }, row.original.name),
      row.original.current_version?.public
        ? h('span', { class: 'shrink-0 rounded-full bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-700 dark:text-emerald-400' }, 'Public')
        : null,
    ]),
  },
  {
    id: 'image',
    header: 'Image',
    cell: ({ row }) => h('code', { class: 'block max-w-56 truncate text-xs text-muted' }, row.original.current_version?.image ?? 'No version'),
  },
  {
    id: 'replicas',
    header: 'Replicas',
    cell: ({ row }) => row.original.current_version?.replica_count ?? 0,
  },
  {
    id: 'port',
    header: 'Port',
    cell: ({ row }) => row.original.current_version?.port ?? '—',
  },
  {
    id: 'access',
    header: 'Access',
    cell: ({ row }) => row.original.current_version?.public ? 'Public' : 'Private',
  },
  {
    accessorKey: 'updated_at',
    header: 'Updated',
    cell: ({ row }) => h(NuxtTime, { datetime: row.original.updated_at, relative: true, class: 'whitespace-nowrap text-xs text-muted' }),
  },
  {
    id: 'actions',
    header: '',
    meta: { class: { th: 'text-right', td: 'text-right' } },
    cell: ({ row }) => props.canRemove
      ? h('div', { class: 'flex justify-end' }, h(UDropdownMenu, {
        items: [[{
          label: 'Remove',
          icon: ICONS.trash,
          color: 'error' as const,
          onSelect: () => { deleteTarget.value = row.original },
        }]],
        size: 'sm',
        content: { align: 'end' },
      }, {
        default: () => h(UButton, {
          icon: ICONS.more,
          color: 'neutral',
          variant: 'ghost',
          size: 'xs',
          'aria-label': `Actions for ${row.original.name}`,
          onClick: (event: MouseEvent) => event.stopPropagation(),
        }),
      }))
      : null,
  },
]

const deleteModalOpen = computed({
  get: () => !!deleteTarget.value,
  set: (value) => { if (!value) deleteTarget.value = null },
})
</script>

<template>
  <UiTable
    :status="status"
    :items="containers"
    :columns="columns"
    disable-header
    selectable
    @select="row => navigateTo(containerUrl(row.original.id))"
  >
    <template #empty>
      <div v-if="hasError" class="flex flex-col items-center justify-center gap-3 py-14 text-center">
        <p class="text-sm text-error">Failed to load containers.</p>
      </div>
      <div v-else class="flex flex-col items-center justify-center gap-3 py-14 text-center">
        <UIcon :name="ICONS.containers" class="size-10 text-muted" />
        <p class="text-muted">{{ containers.length ? 'No matching containers.' : 'No containers yet.' }}</p>
        <p v-if="!containers.length" class="text-sm text-dimmed">Create your first container to run a service in this environment.</p>
      </div>
    </template>
  </UiTable>

  <UModal v-model:open="deleteModalOpen" title="Remove Container">
    <template #body>
      <p class="text-sm">Remove <strong>{{ deleteTarget?.name }}</strong> from this environment?</p>
      <UCheckbox v-model="removeAsDraft" class="mt-4" label="Remove as draft" description="Save this removal without deploying it." />
      <div class="flex justify-end gap-3 pt-4">
        <UButton variant="ghost" color="neutral" @click="deleteTarget = null">Cancel</UButton>
        <UButton color="error" :icon="ICONS.trash" :loading="removing" @click="confirmDelete">{{ removeAsDraft ? 'Remove from draft' : 'Remove and deploy' }}</UButton>
      </div>
    </template>
  </UModal>
</template>
