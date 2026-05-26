<script setup lang="ts">
const open = defineModel<boolean>('open', { required: true });

const props = defineProps<{
  organizationId: string;
  projectId: string;
  timelineId: string;
}>();

interface ResolvedContainer {
  container_id: string;
  container_name: string;
  version_id: string;
  version: number;
  image: string;
}

interface TimelineState {
  id: string;
  branch_id: string;
  timeline: number;
  parent_timeline_id: string | null;
  containers: ResolvedContainer[];
  created_at: string;
}

const loading = ref(false);
const error = ref('');
const state = ref<TimelineState | null>(null);

async function fetchTimeline() {
  loading.value = true;
  error.value = '';
  try {
    state.value = await $fetch<TimelineState>(
      `/api/backend/organization/${props.organizationId}/projects/${props.projectId}/timelines/${props.timelineId}`
    );
  } catch {
    error.value = 'Failed to load timeline state';
  } finally {
    loading.value = false;
  }
}

watch(open, (isOpen) => {
  if (isOpen) fetchTimeline();
});
</script>

<template>
  <UModal v-model:open="open">
    <template #body>
      <div class="flex flex-col gap-4">
        <div class="text-center space-y-1">
          <h2 class="text-lg font-semibold" v-if="state">Revision {{ state.timeline }}</h2>
          <p class="text-sm text-muted" v-if="state">
            {{ new Date(state.created_at).toLocaleString() }}
          </p>
        </div>

        <div v-if="loading" class="py-8 text-center"><UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" /></div>

        <p v-if="error" class="text-sm text-red-500">{{ error }}</p>

        <div v-if="state && !loading" class="space-y-2">
          <div v-if="state.containers.length === 0" class="text-center text-sm text-muted py-4">
            No container deployments at this revision.
          </div>

          <div
            v-for="c in state.containers"
            :key="c.container_id"
            class="rounded-md border border-default bg-default p-3 flex flex-col gap-1"
          >
            <div class="flex items-center justify-between gap-2">
              <span class="font-medium text-sm">{{ c.container_name }}</span>
              <span class="text-xs text-muted">v{{ c.version }}</span>
            </div>
            <code class="text-xs text-muted">{{ c.image }}</code>
          </div>
        </div>

        <div class="flex justify-end pt-2">
          <UButton variant="ghost" color="neutral" @click="open = false">Close</UButton>
        </div>
      </div>
    </template>
  </UModal>
</template>
