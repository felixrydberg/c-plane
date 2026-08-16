<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import type { Environment, TimelineRevision } from '@cplane/sdk'
import { getErrorMessage } from '~/utils/errors'

const store = useStore();
const toast = useToast();

const open = defineModel<boolean>('open', { required: true });
const props = defineProps<{
  parentTimelineId?: string;
}>();
const emit = defineEmits<{ created: [Environment] }>();

const loading = ref(false);
const error = ref('');
const name = ref('');
const timelines = ref<TimelineRevision[]>([]);
const timelinesLoading = ref(false);
const selectedTimelineId = ref<string>('');
const isDropdownOpen = ref(false);
const isPreview = ref(true);

const selectedTimelineLabel = computed(() => {
  const t = timelines.value.find(t => t.id === selectedTimelineId.value);
  return t ? `Revision ${t.timeline} — ${new Date(t.created_at).toLocaleDateString()}` : 'Select a revision';
});

const timelineMenuItems = computed<DropdownMenuItem[][]>(() => {
  const list: DropdownMenuItem[] = timelines.value.map(t => {
    const date = new Date(t.created_at).toLocaleDateString();
    return {
      label: `Revision ${t.timeline} — ${date}`,
      icon: 'i-heroicons:clock',
      onSelect() { selectedTimelineId.value = t.id; },
    };
  });

  return [list];
});

async function fetchTimelines() {
  if (!store.organization?.id || !store.project?.id) return;
  timelinesLoading.value = true;
  try {
    timelines.value = await $fetch(`/api/cplane/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/timelines` as const);
  } catch {
    timelines.value = [];
    error.value = 'Unable to load revisions. Close and reopen this dialog to retry.';
  } finally {
    timelinesLoading.value = false;
  }
}

watch(open, async (isOpen) => {
  if (isOpen) {
    name.value = '';
    isPreview.value = true;
    selectedTimelineId.value = props.parentTimelineId ?? '';
    error.value = '';
    await fetchTimelines();
    if (props.parentTimelineId) {
      if (!timelines.value.some(t => t.id === props.parentTimelineId)) {
        error.value ||= 'The selected parent revision is no longer available.';
      }
      return;
    }
    selectedTimelineId.value = timelines.value[0]?.id ?? '';
    if (!selectedTimelineId.value && !error.value) {
      error.value = 'No revisions are available to base this environment on.';
    }
  }
});

async function handleCreate() {
  if (!store.organization?.id || !store.project?.id || !name.value.trim() || !selectedTimelineId.value) return;

  loading.value = true;
  error.value = '';

  try {
    const body: Record<string, unknown> = { name: name.value.trim(), is_preview: isPreview.value };
    if (selectedTimelineId.value) {
      body.parent_timeline_id = selectedTimelineId.value;
    }

    const created = await $fetch(
      `/api/cplane/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments` as const,
      { method: 'POST', body }
    );

    toast.add({ title: 'Environment created', color: 'success' });
    name.value = '';
    isPreview.value = true;
    selectedTimelineId.value = '';
    open.value = false;
    emit('created', created);
  } catch (e: unknown) {
    error.value = getErrorMessage(e, 'Failed to create environment');
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <UModal v-model:open="open" title="Create Environment" description="Fork a new environment from a timeline revision.">
    <template #body>
      <form class="w-full space-y-4" @submit.prevent="handleCreate">
          <UFormField label="Environment name" required>
            <UInput
              v-model="name"
              placeholder="e.g. staging"
              :disabled="loading"
              autofocus
              class="w-full"
            />
          </UFormField>

          <UFormField v-if="!parentTimelineId" label="Based on">
            <UDropdownMenu
              :items="timelineMenuItems"
              :content="{ align: 'center', collisionPadding: 12 }"
              :ui="{ content: 'w-(--reka-dropdown-menu-trigger-width)' }"
              @open="isDropdownOpen = true"
              @close="isDropdownOpen = false"
            >
              <UButton
                :label="selectedTimelineLabel"
                trailing-icon="i-lucide-chevrons-up-down"
                color="neutral"
                variant="soft"
                block
                :disabled="loading || timelinesLoading"
                class="data-[state=open]:bg-elevated flex-1"
                :ui="{ trailingIcon: 'text-dimmed' }"
              />
            </UDropdownMenu>
          </UFormField>

          <UCheckbox v-model="isPreview" label="Preview environment (delete its revisions when removed)" :disabled="loading" />

          <p v-if="error" class="text-sm text-red-500">{{ error }}</p>

          <div class="flex justify-end gap-3 pt-2">
            <UButton variant="ghost" color="neutral" :disabled="loading" @click="open = false">
              Cancel
            </UButton>
            <UButton type="submit" :loading="loading" :disabled="!name.trim() || !selectedTimelineId">
              Create Environment
            </UButton>
          </div>
        </form>
      </template>
    </UModal>
</template>
