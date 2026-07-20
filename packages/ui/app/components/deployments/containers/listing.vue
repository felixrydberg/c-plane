<script setup lang="ts">
import type { Container } from '@cplane/sdk'
import { ICONS } from '~/utils/icons';

type ContainerWithProject = Container & {
  _projectName?: string;
  _projectId?: string;
}

const props = defineProps<{
  containers: ContainerWithProject[]
  organizationId: string
  projectId: string | null
  environmentId: string | null
  status: string
}>();

const emit = defineEmits<{ refresh: [] }>();

const toast = useToast();
const deleteTarget = ref<ContainerWithProject | null>(null);
const deleting = ref(false);

async function confirmDelete() {
  if (!deleteTarget.value || !props.organizationId || !deleteTarget.value.id) return
  deleting.value = true
  try {
    await $fetch(`/api/cplane/organization/${props.organizationId as ':organization_id'}/containers/${deleteTarget.value.id as ':container_id'}` as const, {
      method: 'DELETE',
      query: { environment_id: props.environmentId ?? undefined },
    });
    toast.add({ title: 'Container removed', color: 'success' });
    deleteTarget.value = null
    emit('refresh');
  } catch {
    toast.add({ title: 'Failed to remove container', color: 'error' });
  } finally {
    deleting.value = false
  }
}

const deleteModalOpen = computed({
  get: () => !!deleteTarget.value,
  set: (v) => { if (!v) deleteTarget.value = null },
})

</script>

<template>
  <div class="overflow-hidden rounded-md border border-dashed border-default bg-transparent">
    <!-- Loading -->
    <div v-if="status === 'pending'" class="flex items-center gap-3 py-12 justify-center text-muted text-sm">
      <UIcon name="i-lucide-loader-circle" class="size-4 animate-spin" />
      Loading containers&hellip;
    </div>

    <!-- Empty -->
    <div v-else-if="containers.length === 0" class="flex flex-col items-center justify-center gap-3 px-6 py-16 text-center">
      <div>
        <p class="text-sm font-medium">No containers in this environment</p>
        <p class="mt-1 text-sm text-muted">Deploy a container to give this environment a running service.</p>
      </div>
    </div>

    <!-- Container rows -->
    <template v-else>
      <div class="hidden grid-cols-[minmax(0,2fr)_minmax(0,1.5fr)_90px_90px_100px_110px_auto] gap-4 border-b border-default/60 px-5 py-3 text-[11px] font-medium uppercase tracking-wide text-muted lg:grid">
        <span>Service</span><span>Image</span><span>Replicas</span><span>Port</span><span>Access</span><span>Updated</span><span />
      </div>
      <NuxtLink
        v-for="c in containers"
        :key="c.id"
        :to="`/${$route.params.organization_slug}/containers/${projectId}/${environmentId}/${c.id}`"
        class="group grid w-full gap-3 border-b border-default/30 px-5 py-4 transition-colors hover:bg-elevated/50 last:border-b-0 lg:grid-cols-[minmax(0,2fr)_minmax(0,1.5fr)_90px_90px_100px_110px_auto] lg:items-center lg:gap-4"
      >
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <span class="text-sm font-medium truncate">{{ c.name }}</span>
            <span v-if="c.current_version?.public" class="rounded-full bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-700 dark:text-emerald-400">Public</span>
          </div>
        </div>
        <code class="truncate font-mono text-xs text-muted">{{ c.current_version?.image ?? 'No version' }}</code>
        <span class="font-mono text-xs text-muted">{{ c.current_version?.replica_count ?? 0 }}</span>
        <span class="font-mono text-xs text-muted">{{ c.current_version?.port ?? '—' }}</span>
        <span class="text-xs text-muted">{{ c.current_version?.public ? 'Public' : 'Private' }}</span>
        <span class="shrink-0 text-xs tabular-nums text-muted">
          <NuxtTime :datetime="c.created_at" relative />
        </span>
        <UButton
          variant="solid"
          size="xs"
          color="error"
          :icon="ICONS.trash"
          @click.prevent.stop="deleteTarget = c"
        >
          Delete
        </UButton>
      </NuxtLink>
    </template>
  </div>

  <UModal v-model:open="deleteModalOpen" title="Delete Container">
    <template #body>
      <p class="text-sm">Remove <strong>{{ deleteTarget?.name }}</strong> from this environment? The container definition will still exist in the project.</p>
      <div class="flex justify-end gap-3 pt-4">
        <UButton variant="ghost" color="neutral" @click="deleteTarget = null">Cancel</UButton>
        <UButton color="error" :icon="ICONS.trash" :loading="deleting" @click="confirmDelete">Delete</UButton>
      </div>
    </template>
  </UModal>
</template>
