<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'

const store = useStore();
const toast = useToast();

const open = defineModel<boolean>('open', { required: true });
const props = defineProps<{
  parentTimelineId?: string;
}>();
const emit = defineEmits<{ created: [{ id: string; name: string; timeline: string; is_default: boolean }] }>();

interface TimelineRevision {
  id: string
  environment_id: string
  timeline: number
  parent_timeline_id: string | null
  pins: Record<string, unknown>
  created_at: string
}

const loading = ref(false);
const error = ref('');
const name = ref('');
const timelines = ref<TimelineRevision[]>([]);
const timelinesLoading = ref(false);
const selectedTimelineId = ref<string>('');
const isDropdownOpen = ref(false);

const selectedTimelineLabel = computed(() => {
  if (!selectedTimelineId.value) return 'Latest revision';
  const t = timelines.value.find(t => t.id === selectedTimelineId.value);
  return t ? `Revision ${t.timeline} — ${new Date(t.created_at).toLocaleDateString()}` : 'Latest revision';
});

const timelineMenuItems = computed<DropdownMenuItem[][]>(() => {
  const list: DropdownMenuItem[] = [{
    label: 'Latest revision',
    icon: 'i-heroicons:star',
    onSelect() { selectedTimelineId.value = ''; },
  }];

  for (const t of timelines.value) {
    const date = new Date(t.created_at).toLocaleDateString();
    list.push({
      label: `Revision ${t.timeline} — ${date}`,
      icon: 'i-heroicons:clock',
      onSelect() { selectedTimelineId.value = t.id; },
    });
  }

  return [list];
});

async function fetchTimelines() {
  if (!store.organization?.id || !store.project?.id) return;
  timelinesLoading.value = true;
  try {
    timelines.value = await $fetch<TimelineRevision[]>(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/timelines`
    );
  } catch {
    timelines.value = [];
  } finally {
    timelinesLoading.value = false;
  }
}

watch(open, async (isOpen) => {
  if (isOpen) {
    name.value = '';
    selectedTimelineId.value = props.parentTimelineId ?? '';
    error.value = '';
    await fetchTimelines();
    if (props.parentTimelineId && !timelines.value.find(t => t.id === props.parentTimelineId)) {
      selectedTimelineId.value = '';
    }
  }
});

async function handleCreate() {
  if (!store.organization?.id || !store.project?.id || !name.value.trim()) return;

  loading.value = true;
  error.value = '';

  try {
    const body: Record<string, unknown> = { name: name.value.trim() };
    if (selectedTimelineId.value) {
      body.parent_timeline_id = selectedTimelineId.value;
    }

    const created = await $fetch<{ id: string; name: string; timeline: string; is_default: boolean }>(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/environments`,
      { method: 'POST', body }
    );

    toast.add({ title: 'Environment created', color: 'success' });
    name.value = '';
    selectedTimelineId.value = '';
    open.value = false;
    emit('created', created);
  } catch (e: unknown) {
    error.value = (e as { data?: { message?: string } })?.data?.message || (e as { message?: string })?.message || 'Failed to create environment';
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

          <p v-if="error" class="text-sm text-red-500">{{ error }}</p>

          <div class="flex justify-end gap-3 pt-2">
            <UButton variant="ghost" color="neutral" :disabled="loading" @click="open = false">
              Cancel
            </UButton>
            <UButton type="submit" :loading="loading" :disabled="!name.trim()">
              Create Environment
            </UButton>
          </div>
        </form>
      </template>
    </UModal>
</template>
