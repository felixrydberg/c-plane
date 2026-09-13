<script setup lang="ts">
import type { Environment, TimelineRevision } from '@cplane/sdk'
import { getErrorMessage } from '~/utils/errors'
import { ICONS } from '~/utils/icons'

const store = useStore();
const toast = useToast();

const open = defineModel<boolean>('open', { required: true });
const props = defineProps<{
  parentTimelineId?: string;
  parentRevision?: TimelineRevision;
}>();
const emit = defineEmits<{ created: [Environment] }>();

const loading = ref(false);
const error = ref('');
const name = ref('');
const timelines = ref<TimelineRevision[]>([]);
const timelineCursors = ref<Array<string | undefined>>([undefined]);
const timelinePage = ref(0);
const nextCursor = ref<string | null>(null);
const timelinesLoading = ref(false);
const parentRevision = ref<TimelineRevision | null>(null);
const selectedTimelineRevision = ref<TimelineRevision | null>(null);
const selectedTimelineId = ref<string>('');
const isPreview = ref(true);
const TIMELINE_PICKER_PAGE_SIZE = 20;

const timelinesUrl = computed(() => store.organization?.id && store.project?.id
  ? `/api/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/timelines` as const
  : '')
const revisionUrl = computed(() => store.organization?.id && store.project?.id && props.parentTimelineId
  ? `/api/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/timelines/${props.parentTimelineId as ':timeline_id'}` as const
  : '')

function revisionLabel(revision: TimelineRevision) {
  const name = revision.name?.trim()
  return name ? `${name} · Revision ${revision.timeline}` : `Revision ${revision.timeline}`
}

function label(revision: TimelineRevision) {
  return `${revisionLabel(revision)} — ${new Date(revision.created_at).toLocaleDateString()}`
}

const selectedTimelineLabel = computed(() => {
  const t = selectedTimelineRevision.value ?? [...timelines.value, parentRevision.value].find(t => t?.id === selectedTimelineId.value);
  return t ? label(t) : 'Select a revision';
});

const timelinePageLabel = computed(() => {
  const revisionNumbers = timelines.value.map(t => t.timeline);
  if (!revisionNumbers.length) return `Page ${timelinePage.value + 1}`;
  const oldest = Math.min(...revisionNumbers);
  const newest = Math.max(...revisionNumbers);
  return `Page ${timelinePage.value + 1} · revisions ${oldest}–${newest}`;
});

function selectTimeline(revision: TimelineRevision) {
  selectedTimelineId.value = revision.id
  selectedTimelineRevision.value = revision
}

async function fetchTimelines(cursor?: string, pageIndex = 0) {
  if (!timelinesUrl.value) return;
  timelinesLoading.value = true;
  try {
    const page = await cplaneFetch(timelinesUrl.value, {
      query: { limit: TIMELINE_PICKER_PAGE_SIZE, ...(cursor ? { cursor } : {}) },
    });
    timelineCursors.value[pageIndex] = cursor;
    timelinePage.value = pageIndex;
    timelines.value = page.data;
    nextCursor.value = page.next_cursor ?? null;
  } catch {
    if (!cursor && pageIndex === 0) timelines.value = [];
    error.value = 'Unable to load revisions. Close and reopen this dialog to retry.';
  } finally {
    timelinesLoading.value = false;
  }
}

async function olderTimelines() {
  if (!nextCursor.value || timelinesLoading.value) return;
  await fetchTimelines(nextCursor.value, timelinePage.value + 1);
}

async function newerTimelines() {
  if (timelinePage.value === 0 || timelinesLoading.value) return;
  await fetchTimelines(timelineCursors.value[timelinePage.value - 1], timelinePage.value - 1);
}

// A supplied revision is fetched directly rather than expected on the current page.
async function fetchParentRevision() {
  if (props.parentRevision) {
    parentRevision.value = props.parentRevision;
    selectedTimelineRevision.value = props.parentRevision;
    return;
  }
  if (!revisionUrl.value) return;
  timelinesLoading.value = true;
  try {
    const resolved = await cplaneFetch(revisionUrl.value);
    parentRevision.value = {
      id: resolved.id,
      environment_id: resolved.environment_id,
      timeline: resolved.timeline,
      name: resolved.name,
      parent_timeline_id: resolved.parent_timeline_id,
      created_at: resolved.created_at,
    };
    selectedTimelineRevision.value = parentRevision.value;
  } catch {
    error.value = 'The selected revision is no longer available.';
  } finally {
    timelinesLoading.value = false;
  }
}

watch(open, async (isOpen) => {
  if (isOpen) {
    name.value = '';
    isPreview.value = true;
    selectedTimelineId.value = props.parentRevision?.id ?? props.parentTimelineId ?? '';
    error.value = '';
    timelines.value = [];
    timelineCursors.value = [undefined];
    timelinePage.value = 0;
    nextCursor.value = null;
    parentRevision.value = null;
    selectedTimelineRevision.value = null;
    if (props.parentRevision || props.parentTimelineId) {
      await fetchParentRevision();
      return;
    }
    await fetchTimelines();
    const firstRevision = timelines.value[0];
    selectedTimelineId.value = firstRevision?.id ?? '';
    selectedTimelineRevision.value = firstRevision ?? null;
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

    const created = await cplaneFetch(
      `/api/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments` as const,
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
  <UModal v-model:open="open" title="Create Environment" description="Create an environment from an existing revision.">
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
            <div class="overflow-hidden rounded-lg border border-default/60">
              <table class="w-full text-sm">
                <thead class="text-left text-xs text-muted">
                  <tr class="border-b border-default/60 bg-elevated/20">
                    <th class="px-3 py-2 font-medium">Revision</th>
                    <th class="w-28 px-3 py-2 font-medium">Created</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-if="!timelines.length">
                    <td colspan="2" class="px-3 py-6 text-center text-muted">
                      {{ timelinesLoading ? 'Loading revisions…' : 'No revisions available.' }}
                    </td>
                  </tr>
                  <tr
                    v-for="t in timelines"
                    :key="t.id"
                    class="cursor-pointer border-b border-default/40 transition-colors last:border-b-0 hover:bg-elevated/30"
                    :class="t.id === selectedTimelineId ? 'bg-elevated/50' : ''"
                    :aria-selected="t.id === selectedTimelineId"
                    tabindex="0"
                    @click="selectTimeline(t)"
                    @keydown.enter.prevent="selectTimeline(t)"
                    @keydown.space.prevent="selectTimeline(t)"
                  >
                    <td class="px-3 py-2">
                      <span class="flex min-w-0 items-center gap-2">
                        <UIcon v-if="t.id === selectedTimelineId" :name="ICONS.check" class="size-4 shrink-0 text-primary" />
                        <span class="truncate">{{ revisionLabel(t) }}</span>
                      </span>
                    </td>
                    <td class="whitespace-nowrap px-3 py-2 text-xs text-muted">{{ new Date(t.created_at).toLocaleDateString() }}</td>
                  </tr>
                </tbody>
                <tfoot class="border-t border-default/60">
                  <tr>
                    <td colspan="2" class="px-2 py-1.5">
                      <div class="flex items-center justify-between gap-2" aria-label="Revision pages">
                        <UButton
                          color="neutral"
                          variant="ghost"
                          size="xs"
                          :icon="ICONS.chevronLeft"
                          :disabled="!nextCursor || timelinesLoading"
                          @click="olderTimelines"
                        >
                          Older
                        </UButton>
                        <span class="text-[11px] text-muted" aria-current="page">{{ timelinePageLabel }}</span>
                        <UButton
                          color="neutral"
                          variant="ghost"
                          size="xs"
                          :trailing-icon="ICONS.chevronRight"
                          :disabled="timelinePage === 0 || timelinesLoading"
                          @click="newerTimelines"
                        >
                          Newer
                        </UButton>
                      </div>
                    </td>
                  </tr>
                </tfoot>
              </table>
            </div>
          </UFormField>
          <UFormField v-else label="Based on">
            <div class="rounded-md border border-default/60 bg-elevated/20 px-3 py-2 text-sm" :class="{ 'text-muted': !parentRevision }">
              {{ parentRevision ? selectedTimelineLabel : 'Loading revision…' }}
            </div>
          </UFormField>

          <div class="space-y-1">
            <UCheckbox v-model="isPreview" label="Preview environment" :disabled="loading" />
            <p class="pl-6 text-xs text-muted">
              Preview environments are temporary. Removing one detaches its revisions; the history stays available for repointing. Use a stable environment when the pin should persist.
            </p>
          </div>

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
