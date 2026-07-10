<script setup lang="ts">
import type { ContainerRow, ContainerVersionRow } from '@cplane/ui-shared/types';
import { ICONS } from '~/utils/icons';

interface Container extends ContainerRow {
  current_version: ContainerVersionRow | null;
  project_id: string | null;
}

interface ContainerVersion extends ContainerVersionRow {
  env_secret_refs?: Record<string, string> | null;
}

interface ContainerWithVersion extends Container {
  current_version: ContainerVersion | null;
}

export interface ContainerWithProject extends ContainerWithVersion {
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
const deleteTarget = ref<ContainerWithProject | null>(null);
const deleting = ref(false);

async function confirmDelete() {
  if (!deleteTarget.value || !props.organizationId || !deleteTarget.value.id) return
  deleting.value = true
  try {
    await $fetch(`/api/backend/organization/${props.organizationId}/containers/${deleteTarget.value.id}?branch_id=${props.branchId ?? ''}`, {
      method: 'DELETE',
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

function secretCount(c: ContainerWithProject): number {
  const refs = c.current_version?.env_secret_refs;
  return refs ? Object.keys(refs).length : 0;
}
</script>

<template>
  <div class="bg-default rounded-lg border border-default/60 overflow-hidden">
    <!-- Loading -->
    <div v-if="status === 'pending'" class="flex items-center gap-3 py-12 justify-center text-muted text-sm">
      <UIcon name="i-lucide-loader-circle" class="size-4 animate-spin" />
      Loading containers&hellip;
    </div>

    <!-- Empty -->
    <div v-else-if="containers.length === 0" class="flex flex-col items-center justify-center gap-3 px-6 py-16 text-center">
      <div>
        <p class="text-sm font-medium">No containers in this branch</p>
        <p class="mt-1 text-sm text-muted">Deploy a container to give this branch a running service.</p>
      </div>
    </div>

    <!-- Container rows -->
    <template v-else>
      <NuxtLink
        v-for="c in containers"
        :key="c.id"
        :to="`/${$route.params.organization_slug}/containers/${projectId}/${branchId}/${c.id}`"
        class="group flex w-full items-center gap-4 border-b border-default/10 px-5 py-4 transition-colors hover:bg-elevated/50 last:border-b-0"
      >
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <span class="text-sm font-medium truncate">{{ c.name }}</span>
            <span v-if="c.current_version?.public" class="rounded-full bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-700 dark:text-emerald-400">Public</span>
          </div>
          <div v-if="c.current_version?.image" class="mt-1 truncate text-xs text-muted">
            <code class="rounded bg-elevated px-1.5 py-0.5">{{ c.current_version.image }}</code>
          </div>
          <div class="mt-2 flex items-center gap-2 text-[11px] font-mono text-muted">
            <template v-if="c.current_version">
              <span>{{ c.current_version.port ? `:${c.current_version.port}` : 'no port' }}</span>
              <span class="text-default/20">&bull;</span>
              <span>{{ c.current_version.replica_count ?? 0 }} replica{{ c.current_version.replica_count === 1 ? '' : 's' }}</span>
              <template v-if="!c.current_version.public">
                <span class="text-default/20">&bull;</span>
                <span>private</span>
              </template>
              <template v-if="secretCount(c) > 0">
                <span class="text-default/20">&bull;</span>
                <span>{{ secretCount(c) }} secret{{ secretCount(c) !== 1 ? 's' : '' }}</span>
              </template>
            </template>
            <span v-else class="italic">no version</span>
          </div>
        </div>

        <span class="hidden shrink-0 text-[11px] tabular-nums text-muted lg:inline">
          <NuxtTime :datetime="c.created_at" relative />
        </span>

        <UIcon name="i-heroicons:chevron-right" class="size-4 text-muted opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />

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
      <p class="text-sm">Remove <strong>{{ deleteTarget?.name }}</strong> from this branch? The container definition will still exist in the project.</p>
      <div class="flex justify-end gap-3 pt-4">
        <UButton variant="ghost" color="neutral" @click="deleteTarget = null">Cancel</UButton>
        <UButton color="error" :icon="ICONS.trash" :loading="deleting" @click="confirmDelete">Delete</UButton>
      </div>
    </template>
  </UModal>
</template>
