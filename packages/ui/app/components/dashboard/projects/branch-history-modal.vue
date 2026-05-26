<script setup lang="ts">
const store = useStore();
const toast = useToast();

const open = defineModel<boolean>('open', { required: true });
const emit = defineEmits<{ updated: [] }>();

interface TimelineRevision {
  id: string
  branch_id: string
  timeline: number
  name: string | null
  parent_timeline_id: string | null
  pins: Record<string, unknown>
  created_at: string
}

const loading = ref(false);
const error = ref('');
const saving = ref('');
const revisions = ref<TimelineRevision[]>([]);

const isCurrentBranch = (rev: TimelineRevision) =>
  rev.id === store.branch?.timeline;

async function fetchRevisions() {
  if (!store.organization?.id || !store.project?.id || !store.branch?.id) return;
  loading.value = true;
  error.value = '';
  try {
    revisions.value = await $fetch<TimelineRevision[]>(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/timelines`,
      { query: { branch_id: store.branch.id } }
    );
  } catch {
    error.value = 'Failed to load revisions';
    revisions.value = [];
  } finally {
    loading.value = false;
  }
}

async function goToRevision(timelineId: string) {
  if (!store.organization?.id || !store.project?.id || !store.branch?.id) return;
  saving.value = timelineId;
  try {
    await $fetch(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/branches/${store.branch.id}`,
      { method: 'PATCH', body: { timeline_id: timelineId } }
    );
    store.branch.timeline = timelineId;
    toast.add({ title: 'Branch updated', color: 'success' });
    await fetchRevisions();
    emit('updated');
  } catch {
    toast.add({ title: 'Failed to update branch', color: 'error' });
  } finally {
    saving.value = '';
  }
}

watch(open, (isOpen) => {
  if (isOpen) fetchRevisions();
});
</script>

<template>
  <UModal v-model:open="open" :title="`Branch History — ${store.branch?.name ?? ''}`" description="Roll back or forward to a previous revision.">
    <template #body>
      <div class="flex flex-col gap-4">
        <div v-if="loading" class="py-4 text-center"><UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" /></div>

        <p v-else-if="error" class="text-sm text-red-500">{{ error }}</p>

        <div v-else class="space-y-2 max-h-80 overflow-y-auto">
          <div
            v-for="rev in revisions"
            :key="rev.id"
            class="rounded-md border border-default p-3 flex items-center justify-between gap-3"
            :class="{ 'bg-primary/5 border-primary/30': isCurrentBranch(rev) }"
          >
            <div class="flex flex-col">
              <span class="text-sm font-medium">
                Revision {{ rev.timeline }}
                <span v-if="rev.name" class="text-muted font-normal text-xs ml-1">— {{ rev.name }}</span>
                <UBadge v-if="isCurrentBranch(rev)" label="current" size="xs" color="primary" variant="soft" class="ml-1" />
              </span>
              <span class="text-xs text-muted">{{ new Date(rev.created_at).toLocaleString() }}</span>
            </div>
            <UButton
              v-if="!isCurrentBranch(rev)"
              size="sm"
              variant="soft"
              :loading="saving === rev.id"
              @click="goToRevision(rev.id)"
            >
              Go to revision
            </UButton>
          </div>
        </div>

        <div class="flex justify-end pt-2">
          <UButton variant="ghost" color="neutral" @click="open = false">Close</UButton>
        </div>
      </div>
    </template>
  </UModal>
</template>
