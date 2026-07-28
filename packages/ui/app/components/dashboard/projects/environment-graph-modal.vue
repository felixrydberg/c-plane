<script setup lang="ts">
import { VueFlow, type VueFlowStore } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import { markRaw, computed, nextTick } from 'vue'
import type { Environment, ResolvedTimeline as SdkResolvedTimeline, TimelineRevision } from '@cplane/sdk'
import DotNode from './environment-graph-dot-node.vue'
import { ICONS } from '~/utils/icons'
import { loadProjectEnvironments } from '~/utils/auth'

const dotNodeType = markRaw(DotNode)

const store = useStore();
const toast = useToast();

const open = defineModel<boolean>('open', { required: true });

const modalContentClass = 'max-w-7xl';

interface ResolvedSecret {
  secret_id: string
  secret_name: string
  version_id: string
  version: number
}

interface EnvironmentMeta {
  id: string
  name: string
  isDefault: boolean
  exists: boolean
  isDraft: boolean
  isDeployed: boolean
}

interface NodeMeta {
  environmentName: string
  color: string
  environments: EnvironmentMeta[]
}

const loading = ref(false);
const error = ref('');
const nodes = ref<any[]>([]);
const edges = ref<any[]>([]);

const createEnvironmentModalOpen = ref(false);
const createEnvironmentFromRevisionId = ref('');
const repointModalOpen = ref(false);
const repointRevisionId = ref('');
const repointEnvironmentId = ref('');
const allEnvironments = ref<Environment[]>([]);

const removeEnvironmentId = ref('');
const removeEnvironmentName = ref('');
const removeModalOpen = ref(false);
const removeEnvironmentIsPreview = computed(() => allEnvironments.value.find(b => b.id === removeEnvironmentId.value)?.is_preview ?? false);
const renameEnvironmentId = ref('');
const renameEnvironmentName = ref('');
const renameModalOpen = ref(false);
const renamingEnvironment = ref(false);

// --- Detail panel state ---
const selectedRevisionId = ref<string | null>(null);
const selectedTimelineData = ref<(SdkResolvedTimeline & { secrets: ResolvedSecret[] }) | null>(null);
const detailLoading = ref(false);
const detailError = ref('');

const nodeMetaMap = ref<Map<string, NodeMeta>>(new Map());

const selectedNodeMeta = computed<NodeMeta | null>(() => {
  if (!selectedRevisionId.value) return null;
  return nodeMetaMap.value.get(selectedRevisionId.value) ?? null;
});

const graph = ref<VueFlowStore | null>(null);

async function focusSelectedRevision() {
  if (!selectedRevisionId.value || !graph.value) return;
  await graph.value.fitView({
    nodes: [selectedRevisionId.value],
    minZoom: 1,
    maxZoom: 1,
    duration: 0,
  });
}

function onPaneReady(instance: VueFlowStore) {
  graph.value = instance;
  void focusSelectedRevision();
}

function onNodeClick({ node }: { node: { id: string } }) {
  if (selectedRevisionId.value === node.id) {
    deselectRevision();
  } else {
    selectRevision(node.id);
  }
}

function deselectRevision() {
  selectedRevisionId.value = null;
  selectedTimelineData.value = null;
  detailError.value = '';
}

async function selectRevision(revisionId: string) {
  if (!store.organization?.id || !store.project?.id) return;

  selectedRevisionId.value = revisionId;
  detailLoading.value = true;
  detailError.value = '';
  selectedTimelineData.value = null;

  try {
    const data = await $fetch(`/api/cplane/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/timelines/${revisionId as ':timeline_id'}` as const);
    selectedTimelineData.value = {
      id: data.id,
      environment_id: data.environment_id ?? null,
      timeline: data.timeline,
      name: data.name ?? null,
      parent_timeline_id: data.parent_timeline_id ?? null,
      containers: data.containers || [],
      secrets: [],
      created_at: data.created_at,
    };
  } catch {
    detailError.value = 'Failed to load revision details';
  } finally {
    detailLoading.value = false;
  }
}

// Update selected node data in nodes when selection changes
watch(selectedRevisionId, (newId) => {
  nodes.value = nodes.value.map(n => ({
    ...n,
    data: { ...n.data, isSelected: n.id === newId },
  }));
});

// --- Environment management (called from sidebar) ---
function onCreateEnvironment(revisionId: string) {
  createEnvironmentFromRevisionId.value = revisionId;
  createEnvironmentModalOpen.value = true;
}

function onRepointEnvironment(revisionId: string) {
  repointRevisionId.value = revisionId;
  repointModalOpen.value = true;
}

function onRemoveEnvironment(environmentId: string) {
  const environment = allEnvironments.value.find(b => b.id === environmentId);
  removeEnvironmentId.value = environmentId;
  removeEnvironmentName.value = environment?.name ?? environmentId;
  removeModalOpen.value = true;
}

function onRenameEnvironment(environmentId: string) {
  const environment = allEnvironments.value.find(b => b.id === environmentId);
  renameEnvironmentId.value = environmentId;
  renameEnvironmentName.value = environment?.name ?? '';
  renameModalOpen.value = true;
}

async function onConfirmRenameEnvironment() {
  if (!store.organization?.id || !store.project?.id || !renameEnvironmentId.value || !renameEnvironmentName.value.trim()) return;

  renamingEnvironment.value = true;
  try {
    const updated = await $fetch(
      `/api/cplane/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments/${renameEnvironmentId.value as ':environment_id'}` as const,
      { method: 'PATCH', body: { name: renameEnvironmentName.value.trim() } }
    );
    if (store.environment?.id === updated.id) store.environment = updated;
    store.environments = store.environments.map(environment => environment.id === updated.id ? updated : environment);
    renameModalOpen.value = false;
    toast.add({ title: 'Environment renamed', color: 'success' });
    refresh();
  } catch (error) {
    toast.add({ title: (error as { data?: { message?: string } })?.data?.message || 'Failed to rename environment', color: 'error' });
  } finally {
    renamingEnvironment.value = false;
  }
}

async function onConfirmRemoveEnvironment() {
  if (!store.organization?.id || !store.project?.id || !removeEnvironmentId.value) return;
  try {
    await $fetch(`/api/cplane/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments/${removeEnvironmentId.value as ':environment_id'}` as const, { method: 'DELETE' });
    toast.add({ title: 'Environment removed', color: 'success' });
    const deletedId = removeEnvironmentId.value;
    removeModalOpen.value = false;
    removeEnvironmentId.value = '';
    removeEnvironmentName.value = '';
    await loadProjectEnvironments(
      store.project.id,
      store.environment?.id === deletedId ? undefined : store.environment?.id,
    );
    await loadGraph(false);
  } catch {
    toast.add({ title: 'Failed to remove environment', color: 'error' });
  }
}

async function onEnvironmentCreated(environment: Environment) {
  store.environments = [...store.environments.filter(b => b.id !== environment.id), environment];
  store.environments_project_id = store.project?.id ?? store.environments_project_id;
  await loadGraph(true);
}

async function onSelectRepointEnvironment() {
  if (!store.organization?.id || !store.project?.id || !repointEnvironmentId.value) return;
  try {
    await $fetch(`/api/cplane/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments/${repointEnvironmentId.value as ':environment_id'}` as const, { method: 'PATCH', body: { draft_timeline_id: repointRevisionId.value, deployed_timeline_id: repointRevisionId.value } });
    toast.add({ title: 'Environment repointed', color: 'success' });
    store.refreshKey++;
    repointModalOpen.value = false;
    repointEnvironmentId.value = '';
    refresh();
  } catch {
    toast.add({ title: 'Failed to repoint environment', color: 'error' });
  }
}

const environmentColors = ['#3b82f6', '#22c55e', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4'];

async function loadGraph(preserveSelection = false) {
  if (!store.organization?.id || !store.project?.id) return;
  loading.value = true;
  error.value = '';

  // Preserve selection for refreshes triggered by sub-actions (environment create, repoint, delete)
  const savedRevisionId = preserveSelection ? selectedRevisionId.value : null;
  if (!preserveSelection) {
    selectedRevisionId.value = null;
    selectedTimelineData.value = null;
    detailError.value = '';
  }

  try {
    const environments = await $fetch(`/api/cplane/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments` as const);

    allEnvironments.value = environments;

    const isDefaultMap = new Map<string, boolean>();
    environments.forEach(b => isDefaultMap.set(b.id, b.is_default));

    const allTimelines = await $fetch(`/api/cplane/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/timelines` as const);

    const environmentTimelines = new Map<string, TimelineRevision[]>();
    for (const t of allTimelines) {
      if (!t.environment_id) continue;
      const list = environmentTimelines.get(t.environment_id) || [];
      list.push(t);
      environmentTimelines.set(t.environment_id, list);
    }

    const environmentDraftTimelineMap = new Map<string, string>();
    const environmentDeployedTimelineMap = new Map<string, string>();
    const colorMap = new Map<string, string>();
    const environmentNames = new Map<string, string>();
    const environmentExists = new Map<string, boolean>();

    environments.forEach((b, i) => {
      environmentDraftTimelineMap.set(b.id, b.draft_timeline);
      environmentDeployedTimelineMap.set(b.id, b.deployed_timeline);
      colorMap.set(b.id, environmentColors[i % environmentColors.length]);
      environmentNames.set(b.id, `${b.name} (${b.is_preview ? 'Preview' : 'Stable'})`);
      environmentExists.set(b.id, true);
    });

    for (const environmentId of environmentTimelines.keys()) {
      if (environmentNames.has(environmentId)) continue;
      environmentNames.set(environmentId, 'Deleted Environment');
      colorMap.set(environmentId, '#6b7280');
      environmentExists.set(environmentId, false);
      const revs = environmentTimelines.get(environmentId)!;
      const head = revs.reduce((a, b) => a.timeline > b.timeline ? a : b);
      environmentDraftTimelineMap.set(environmentId, head.id);
      environmentDeployedTimelineMap.set(environmentId, head.id);
    }

    const allRevs: { environmentId: string; environmentName: string; rev: TimelineRevision }[] = [];
    for (const [environmentId, revs] of environmentTimelines) {
      const name = environmentNames.get(environmentId) || environmentId;
      for (const rev of revs) {
        allRevs.push({ environmentId, environmentName: name, rev });
      }
    }

    const Y_SPACING = 120;
    const X_SPACING = 360;

    const mainEnvironment = environments.find(b => b.name === 'main') || environments[0];
    const revisionsById = new Map(allRevs.map(({ rev }) => [rev.id, rev]));
    const mainRevs: TimelineRevision[] = [];
    let currentMainRevision = mainEnvironment ? revisionsById.get(mainEnvironment.draft_timeline) : undefined;
    while (currentMainRevision) {
      mainRevs.unshift(currentMainRevision);
      currentMainRevision = currentMainRevision.parent_timeline_id
        ? revisionsById.get(currentMainRevision.parent_timeline_id)
        : undefined;
    }

    const childrenMap = new Map<string, TimelineRevision[]>();
    for (const { rev } of allRevs) {
      if (rev.parent_timeline_id) {
        const list = childrenMap.get(rev.parent_timeline_id) || [];
        list.push(rev);
        childrenMap.set(rev.parent_timeline_id, list);
      }
    }

    const xPosMap = new Map<string, number>();
    const yLaneMap = new Map<string, number>();

    mainRevs.forEach((r, i) => {
      xPosMap.set(r.id, i * X_SPACING);
      yLaneMap.set(r.id, 0);
    });

    let nextDownLane = 1;
    let nextUpLane = -1;
    for (const mainRev of mainRevs) {
      const forked = childrenMap.get(mainRev.id)?.filter(c => !mainRevs.some(mr => mr.id === c.id)) || [];
      let useUp = true;
      for (const f of forked) {
        const lane = useUp ? nextUpLane-- : nextDownLane++;
        const baseX = xPosMap.get(mainRev.id)! + X_SPACING;
        let current = f;
        let depth = 0;
        while (current) {
          xPosMap.set(current.id, baseX + (depth * X_SPACING));
          yLaneMap.set(current.id, lane);
          depth++;
          const next = childrenMap.get(current.id)?.[0];
          current = next || null;
        }
        useUp = !useUp;
      }
    }

    const newNodes: any[] = [];
    const newEdges: any[] = [];
    const processedEdges = new Set<string>();

    const minLane = Math.min(...yLaneMap.values(), 0);
    const maxLane = Math.max(...yLaneMap.values(), 0);
    const centerY = ((maxLane + minLane) / 2) * Y_SPACING;

    const seen = new Set<string>();
    const metaMap = new Map<string, NodeMeta>();

    for (const { rev, environmentName } of allRevs) {
      if (seen.has(rev.id)) continue;
      seen.add(rev.id);

      const x = xPosMap.get(rev.id) || 0;
      const lane = yLaneMap.get(rev.id) || 0;
      const y = lane * Y_SPACING - centerY;

      const environmentLabels: string[] = [];
      const pointingEnvironments: EnvironmentMeta[] = [];
      for (const [id, name] of environmentNames) {
        const isDraft = environmentDraftTimelineMap.get(id) === rev.id;
        const isDeployed = environmentDeployedTimelineMap.get(id) === rev.id;
        if (!isDraft && !isDeployed) continue;
        const exists = environmentExists.get(id) ?? false;
        if (exists) {
          environmentLabels.push(isDraft === isDeployed ? name : `${name} (${isDraft ? 'Draft' : 'Deployed'})`);
        }
        pointingEnvironments.push({
          id,
          name: exists ? name : 'Deleted Environment',
          isDefault: isDefaultMap.get(id) || false,
          exists,
          isDraft,
          isDeployed,
        });
      }

      const color = colorMap.get(rev.environment_id) || '#888';

      metaMap.set(rev.id, { environmentName, color, environments: pointingEnvironments });

      newNodes.push({
        id: rev.id,
        type: 'dot',
        position: { x, y },
        data: {
          label: rev.name || `Revision ${rev.timeline}`,
          environmentName,
          timeline: rev.timeline,
          date: new Date(rev.created_at).toLocaleString(),
          color,
          isHead: pointingEnvironments.some(b => b.exists),
          environments: pointingEnvironments,
          environmentLabels,
          isSelected: false,
        },
      });

      if (rev.parent_timeline_id) {
        const edgeId = `${rev.parent_timeline_id}-${rev.id}`;
        if (!processedEdges.has(edgeId)) {
          newEdges.push({
            id: edgeId,
            source: rev.parent_timeline_id,
            target: rev.id,
            type: 'default',
            style: { strokeDasharray: '5,5' },
            animated: true,
          });
          processedEdges.add(edgeId);
        }
      }
    }

    nodeMetaMap.value = metaMap;
    nodes.value = newNodes;
    edges.value = newEdges;

    const revisionToSelect = preserveSelection
      ? savedRevisionId
      : store.environment?.draft_timeline ?? null;
    if (revisionToSelect && newNodes.some(node => node.id === revisionToSelect)) {
      if (revisionToSelect !== selectedRevisionId.value) {
        selectedRevisionId.value = revisionToSelect;
      }
      selectRevision(revisionToSelect);
    }
  } catch {
    error.value = 'Failed to load graph';
  } finally {
    loading.value = false;
    if (!preserveSelection) {
      await nextTick();
      await focusSelectedRevision();
    }
  }
}

function refresh() {
  loadGraph(true);
}

function navigateAndClose(to: string) {
  open.value = false
  navigateTo(to)
}

watch(open, (isOpen) => {
  if (isOpen) loadGraph(false);
});
</script>

<template>
  <UModal v-model:open="open" title="Environment Graph" :description="`${store.project?.name ?? ''} — revision timeline`" :ui="{ content: modalContentClass }">
    <template #body>
      <div class="flex flex-col gap-4 relative" style="height: 600px">
        <div v-if="loading" class="flex items-center justify-center h-full"><UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" /></div>
        <p v-else-if="error" class="text-sm text-error">{{ error }}</p>
        <div v-else class="flex gap-0 overflow-hidden flex-1 min-h-0">
          <!-- Graph area -->
          <div class="flex-1 min-w-0 rounded-lg border border-default bg-default/30">
            <VueFlow
              :nodes="nodes"
              :edges="edges"
              :default-viewport="{ x: 0, y: 0, zoom: 1 }"
              :min-zoom="0.1"
              :max-zoom="4"
              :nodes-draggable="false"
              :nodes-connectable="false"
              :edges-updatable="false"
              :node-types="{ dot: dotNodeType }"
              @node-click="onNodeClick"
              @pane-ready="onPaneReady"
            >
              <Background :gap="20" :size="1" />
            </VueFlow>
          </div>

          <!-- Sidebar -->
          <Transition name="sidebar">
            <div
              v-if="selectedRevisionId"
              class="flex-shrink-0 overflow-hidden min-w-0 h-full"
            >
              <div class="w-80 h-full bg-default flex flex-col overflow-hidden">
              <!-- Loading -->
              <div v-if="detailLoading" class="flex-1 flex items-center justify-center">
                <div class="flex flex-col items-center gap-3">
                  <UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" />
                  <p class="text-xs text-muted">Loading revision...</p>
                </div>
              </div>

              <!-- Error -->
              <div v-else-if="detailError" class="flex-1 flex flex-col items-center justify-center gap-3 p-5">
                <p class="text-sm text-muted text-center">{{ detailError }}</p>
                <UButton size="xs" variant="soft" color="neutral" @click="selectRevision(selectedRevisionId!)">Retry</UButton>
              </div>

              <!-- Resolved -->
              <template v-else-if="selectedTimelineData">
                <!-- Header -->
                <div class="pl-5 pb-3">
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <h3 class="text-base font-semibold truncate">
                        {{ selectedTimelineData.name || `Revision ${selectedTimelineData.timeline}` }}
                      </h3>
                      <p class="text-xs text-muted/70 mt-0.5">
                        Created {{ new Date(selectedTimelineData.created_at).toLocaleString(undefined, { month: 'short', day: 'numeric', year: 'numeric', hour: 'numeric', minute: '2-digit' }) }}
                      </p>
                    </div>
                    <!-- size="sm" -->
                    <UButton
                      variant="ghost"
                      color="neutral"
                      :icon="ICONS.sidebarCloseRight"
                      aria-label="Close sidebar"
                      @click="deselectRevision"
                    />
                  </div>
                </div>

                <div class="flex-1 overflow-y-auto pl-5 pb-5 space-y-5">
                  <!-- Environments -->
                  <div v-if="selectedNodeMeta && selectedNodeMeta.environments.length > 0">
                    <div class="text-xs font-medium text-muted mb-1.5 uppercase tracking-wider">Environments</div>
                    <div class="border border-default/40 rounded-lg overflow-hidden">
                      <button
                        v-for="b in selectedNodeMeta.environments"
                        :key="b.id"
                        :disabled="!b.exists"
                        class="group w-full text-left flex items-center gap-3 px-4 py-2.5 hover:bg-elevated/50 transition-colors border-b border-default/10 last:border-b-0 disabled:opacity-50"
                        @click="b.exists ? navigateAndClose(`/${store.organization?.slug}/containers/${store.project?.id}/${b.id}`) : undefined"
                      >
                        <div class="size-7 shrink-0 rounded-md bg-elevated flex items-center justify-center">
                          <UIcon name="i-heroicons:folder" class="size-3.5 text-muted" />
                        </div>
                        <span class="text-sm capitalize flex-1 truncate">{{ b.name }}</span>
                        <span v-if="b.isDefault" class="text-[10px] text-muted">default</span>
                        <span v-if="b.isDraft && !b.isDeployed" class="text-[10px] text-warning">Draft</span>
                        <span v-else-if="b.isDeployed && !b.isDraft" class="text-[10px] text-muted">Deployed</span>
                        <UIcon v-if="b.exists" name="i-heroicons:chevron-right" class="size-3.5 text-muted opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />
                        <UButton
                          v-if="b.exists"
                          variant="solid"
                          size="xs"
                          color="neutral"
                          :icon="ICONS.pencil"
                          @click.prevent.stop="onRenameEnvironment(b.id)"
                        >
                          Rename
                        </UButton>
                        <UButton
                          v-if="!b.isDefault && b.exists"
                          variant="solid"
                          size="xs"
                          color="error"
                          :icon="ICONS.trash"
                          @click.prevent.stop="onRemoveEnvironment(b.id)"
                        >
                          Delete
                        </UButton>
                      </button>
                    </div>
                  </div>

                  <!-- Containers -->
                  <div v-if="selectedTimelineData.containers.length > 0">
                    <div class="text-xs font-medium text-muted mb-1.5 uppercase tracking-wider">Containers</div>
                    <div class="border border-default/40 rounded-lg overflow-hidden">
                      <button
                        v-for="c in selectedTimelineData.containers"
                        :key="c.container_id"
                        :disabled="!selectedTimelineData.environment_id"
                        class="group w-full text-left flex items-center gap-3 px-4 py-2.5 hover:bg-elevated/50 transition-colors border-b border-default/10 last:border-b-0 disabled:opacity-50"
                        @click="selectedTimelineData.environment_id ? navigateAndClose(`/${store.organization?.slug}/containers/${store.project?.id}/${selectedTimelineData.environment_id}/${c.container_id}?revision=${selectedTimelineData.id}`) : undefined"
                      >
                        <div class="min-w-0 flex-1">
                          <span class="text-sm font-medium truncate block">{{ c.container_name }}</span>
                          <div class="flex items-center gap-2 mt-0.5 text-[11px] text-muted font-mono">
                            <code class="bg-elevated px-1 py-0.5 rounded">{{ c.image }}</code>
                            <span class="text-default/20">&bull;</span>
                            <span>v{{ c.version }}</span>
                          </div>
                        </div>
                        <UIcon name="i-heroicons:chevron-right" class="size-3.5 text-muted opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />
                      </button>
                    </div>
                  </div>

                  <!-- Secrets -->
                  <div v-if="selectedTimelineData.secrets.length > 0">
                    <div class="text-xs font-medium text-muted mb-1.5 uppercase tracking-wider">Secrets</div>
                    <div class="border border-default/40 rounded-lg overflow-hidden">
                      <button
                        v-for="s in selectedTimelineData.secrets"
                        :key="s.secret_id"
                        class="group w-full text-left flex items-center gap-3 px-4 py-2.5 hover:bg-elevated/50 transition-colors border-b border-default/10 last:border-b-0"
                        @click="navigateAndClose(`/${store.organization?.slug}/secrets/${store.project?.id}`)"
                      >
                        <div class="min-w-0 flex-1">
                          <span class="text-sm font-medium font-mono truncate block">{{ s.secret_name }}</span>
                          <span class="text-[11px] text-muted font-mono mt-0.5 block">v{{ s.version }}</span>
                        </div>
                        <UIcon name="i-heroicons:chevron-right" class="size-3.5 text-muted opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />
                      </button>
                    </div>
                  </div>

                  <!-- Fully empty -->
                  <div
                    v-if="selectedTimelineData.containers.length === 0 && selectedTimelineData.secrets.length === 0 && (!selectedNodeMeta || selectedNodeMeta.environments.length === 0)"
                    class="py-12 text-center"
                  >
                    <p class="text-sm text-muted/50">Nothing pinned at this revision.</p>
                  </div>
                </div>

                <!-- Footer -->
                <div class="pl-5 flex gap-2.5">
                  <UButton
                    variant="solid"
                    size="sm"
                    color="neutral"
                    :icon="ICONS.plus"
                    block
                    @click="onCreateEnvironment(selectedRevisionId!)"
                  >
                    Create environment
                  </UButton>
                  <UButton
                    variant="solid"
                    size="sm"
                    color="neutral"
                    :icon="ICONS.arrowTopRight"
                    block
                    @click="onRepointEnvironment(selectedRevisionId!)"
                  >
                    Repoint
                  </UButton>
                </div>
              </template>

              <!-- Fallback -->
              <div v-else class="flex-1 flex items-center justify-center">
                <UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" />
              </div>
              </div>
            </div>
          </Transition>
        </div>

        <DashboardProjectsCreateEnvironmentModal
          v-model:open="createEnvironmentModalOpen"
          :parent-timeline-id="createEnvironmentFromRevisionId"
          @created="onEnvironmentCreated"
        />

        <UModal v-model:open="repointModalOpen" title="Repoint Environment" :ui="{ content: 'max-w-sm' }">
          <template #body>
            <p class="text-sm text-muted mb-4">Choose a environment to repoint to this revision.</p>
            <div class="border border-default/40 rounded-lg overflow-hidden max-h-48 overflow-y-auto">
              <button
                v-for="b in allEnvironments.filter(b => !b.is_preview)"
                :key="b.id"
                class="group w-full flex items-center gap-3 px-4 py-2.5 text-left hover:bg-elevated/50 transition-colors border-b border-default/10 last:border-b-0"
                @click="repointEnvironmentId = b.id; onSelectRepointEnvironment()"
              >
                <div class="size-7 shrink-0 rounded-md bg-elevated flex items-center justify-center">
                  <UIcon name="i-heroicons:folder" class="size-3.5 text-muted" />
                </div>
                <span class="text-sm capitalize flex-1">{{ b.name }} (Stable)</span>
                <UIcon name="i-heroicons:chevron-right" class="size-3.5 text-muted opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />
              </button>
              <p v-if="!allEnvironments.some(b => !b.is_preview)" class="text-sm text-muted px-4 py-3">No non-preview environments available.</p>
            </div>
            <div class="flex justify-end gap-3 pt-3">
              <UButton variant="ghost" color="neutral" @click="repointModalOpen = false; repointEnvironmentId = ''">Cancel</UButton>
            </div>
          </template>
        </UModal>

        <UModal v-model:open="removeModalOpen" title="Remove Environment" :ui="{ content: 'max-w-sm' }">
          <template #body>
            <p class="text-sm">
              Are you sure you want to remove the environment <strong class="capitalize">{{ removeEnvironmentName }}</strong>? {{ removeEnvironmentIsPreview ? 'Its timeline revisions will be deleted.' : 'Timeline revisions will be preserved and can be repointed to.' }}
            </p>
            <div class="flex justify-end gap-3 pt-4">
              <UButton variant="ghost" color="neutral" @click="removeModalOpen = false">Cancel</UButton>
              <UButton color="error" @click="onConfirmRemoveEnvironment">Remove</UButton>
            </div>
          </template>
        </UModal>

        <UModal v-model:open="renameModalOpen" title="Rename Environment" :ui="{ content: 'max-w-sm' }">
          <template #body>
            <form class="space-y-4" @submit.prevent="onConfirmRenameEnvironment">
              <UFormField label="Environment name" required>
                <UInput v-model="renameEnvironmentName" :disabled="renamingEnvironment" autofocus class="w-full" />
              </UFormField>
              <div class="flex justify-end gap-3">
                <UButton color="neutral" variant="ghost" :disabled="renamingEnvironment" @click="renameModalOpen = false">Cancel</UButton>
                <UButton type="submit" :icon="ICONS.check" color="primary" :loading="renamingEnvironment" :disabled="!renameEnvironmentName.trim()">Save</UButton>
              </div>
            </form>
          </template>
        </UModal>
      </div>
    </template>
  </UModal>
</template>

<style scoped>
.sidebar-enter-active,
.sidebar-leave-active {
  transition: width 0.25s ease, opacity 0.2s ease;
  overflow: hidden;
}
.sidebar-enter-from,
.sidebar-leave-to {
  width: 0 !important;
  opacity: 0;
}
.sidebar-enter-to,
.sidebar-leave-from {
  width: 320px;
  opacity: 1;
}
</style>
