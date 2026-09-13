<script setup lang="ts">
import { VueFlow, type Edge, type Node, type VueFlowStore } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { ControlButton, Controls } from '@vue-flow/controls'
import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import '@vue-flow/controls/dist/style.css'
import { markRaw, computed, nextTick } from 'vue'
import type { Environment, TimelineRevision, components } from '@cplane/sdk'
import DotNode from './environment-graph-dot-node.vue'
import { ICONS } from '~/utils/icons'
import { loadProjectEnvironments } from '~/utils/auth'
import { syncEnvironment } from '~/utils/environments'

const dotNodeType = markRaw(DotNode)

const store = useStore();
const toast = useToast();
const route = useRoute();

const open = defineModel<boolean>('open', { required: true });

const modalContentClass = 'max-w-7xl';
const GRAPH_PAGE_SIZE = 12;
const HISTORY_EDGE_STYLE = { stroke: '#6b7280', strokeWidth: 2, strokeDasharray: '6,4' };

interface EnvironmentMeta {
  id: string
  name: string
  isDefault: boolean
  isPreview: boolean
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
const nodes = ref<Node[]>([]);
const edges = ref<Edge[]>([]);

// The graph is a book of fixed revision-order pages. Opening from an
// environment anchors the page that contains its draft revision.
const graphPage = ref(0);
const graphBook = ref<components['schemas']['TimelineGraphNode'][]>([]);
const graphTotalPages = ref(1);
const hasOlderRevisions = ref(false);
const newerRevisionIds = ref<Set<string>>(new Set());

const currentRevisionId = computed(() => {
  const revision = route.query.revision;
  return typeof revision === 'string' ? revision : (store.environment?.draft_timeline ?? null);
});

const graphPageLabel = computed(() => {
  const revisionNumbers = nodes.value
    .filter(node => !node.id.startsWith('boundary-'))
    .map(node => node.data?.timeline)
    .filter((timeline): timeline is number => typeof timeline === 'number');
  const page = `Page ${graphPage.value + 1} of ${graphTotalPages.value}`;
  if (!revisionNumbers.length) return page;
  const oldest = Math.min(...revisionNumbers);
  const newest = Math.max(...revisionNumbers);
  return `${page} · revisions ${oldest}–${newest}`;
});

const createEnvironmentModalOpen = ref(false);
const createEnvironmentFromRevisionId = ref('');
const repointModalOpen = ref(false);
const repointRevisionId = ref('');
const repointEnvironmentId = ref('');
const repointing = ref(false);
const allEnvironments = ref<Environment[]>([]);

const removeEnvironmentId = ref('');
const removeEnvironmentName = ref('');
const removeModalOpen = ref(false);
const renameEnvironmentId = ref('');
const renameEnvironmentName = ref('');
const renameModalOpen = ref(false);
const renamingEnvironment = ref(false);

// --- Detail panel state ---
const selectedRevisionId = ref<string | null>(null);
const selectedTimelineData = ref<TimelineRevision | null>(null);
const timelineDataMap = ref<Map<string, TimelineRevision>>(new Map());

const nodeMetaMap = ref<Map<string, NodeMeta>>(new Map());
const selectedNodeMetaSnapshot = ref<NodeMeta | null>(null);
const selectedRevisionPage = ref<number | null>(null);

const selectedNodeMeta = computed<NodeMeta | null>(() => {
  if (!selectedRevisionId.value) return null;
  return nodeMetaMap.value.get(selectedRevisionId.value) ?? selectedNodeMetaSnapshot.value;
});
const selectedRevisionVisible = computed(() =>
  !!selectedRevisionId.value && nodes.value.some(node => node.id === selectedRevisionId.value),
);

const graph = ref<VueFlowStore | null>(null);

const visibleGraphNodeIds = computed(() => nodes.value
    .map(node => node.id));

async function fitVisibleRevisions() {
  if (!graph.value || !visibleGraphNodeIds.value.length) return;

  await graph.value.fitView({
    nodes: visibleGraphNodeIds.value,
    minZoom: 0.2,
    maxZoom: 1.2,
    padding: 0.08,
    duration: 0,
  });
}

async function focusSelectedRevision() {
  if (!graph.value || !nodes.value.length) return;
  await fitVisibleRevisions();
}

function onPaneReady(instance: VueFlowStore) {
  graph.value = instance;
  void focusSelectedRevision();
}

function onNodeClick({ node }: { node: { id: string } }) {
  if (node.id.startsWith('boundary-')) {
    void loadGraph(false, false, nodes.value.find(item => item.id === node.id)?.data.targetRevisionId);
    return;
  }
  if (selectedRevisionId.value === node.id) {
    deselectRevision();
  } else {
    selectRevision(node.id);
  }
}

function deselectRevision() {
  selectedRevisionId.value = null;
  selectedTimelineData.value = null;
  selectedNodeMetaSnapshot.value = null;
  selectedRevisionPage.value = null;
}

function selectRevision(revisionId: string) {
  const revision = timelineDataMap.value.get(revisionId);
  if (!revision) return;
  selectedRevisionId.value = revisionId;
  selectedTimelineData.value = revision;
  selectedNodeMetaSnapshot.value = nodeMetaMap.value.get(revisionId) ?? null;
  selectedRevisionPage.value = graphPage.value;
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
  repointEnvironmentId.value = '';
  repointModalOpen.value = true;
}

const repointEnvironment = computed(() => allEnvironments.value.find(environment => environment.id === repointEnvironmentId.value));
const repointRevisionLabel = computed(() => {
  const revision = selectedTimelineData.value?.id === repointRevisionId.value ? selectedTimelineData.value : null;
  if (!revision) return 'Selected revision';
  return revision.name?.trim() ? `${revision.name} · Revision ${revision.timeline}` : `Revision ${revision.timeline}`;
});

function selectRepointEnvironment(environmentId: string) {
  repointEnvironmentId.value = environmentId;
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
    const updated = await cplaneFetch(
      `/api/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments/${renameEnvironmentId.value as ':environment_id'}` as const,
      { method: 'PATCH', body: { name: renameEnvironmentName.value.trim() } }
    );
    syncEnvironment(store, updated);
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
    await cplaneFetch(`/api/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments/${removeEnvironmentId.value as ':environment_id'}` as const, { method: 'DELETE' });
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

async function onConfirmRepointEnvironment() {
  if (!store.organization?.id || !store.project?.id || !repointEnvironmentId.value) return;
  repointing.value = true;
  try {
    const updated = await cplaneFetch(`/api/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments/${repointEnvironmentId.value as ':environment_id'}` as const, { method: 'PATCH', body: { draft_timeline_id: repointRevisionId.value, deployed_timeline_id: repointRevisionId.value } });
    syncEnvironment(store, updated);
    toast.add({ title: 'Environment repointed', color: 'success' });
    store.refreshKey++;
    repointModalOpen.value = false;
    repointEnvironmentId.value = '';
    refresh();
  } catch {
    toast.add({ title: 'Failed to repoint environment', color: 'error' });
  } finally {
    repointing.value = false;
  }
}

const environmentColors = ['#3b82f6', '#22c55e', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4'];

async function loadGraph(preserveSelection = false, resetPage = false, anchorRevision?: string | null) {
  if (!store.organization?.id || !store.project?.id) return;
  if (resetPage) {
    graphPage.value = 0;
    graphBook.value = [];
  }
  loading.value = true;
  error.value = '';

  // Preserve selection for refreshes triggered by sub-actions (environment create, repoint, delete)
  const savedRevisionId = preserveSelection ? selectedRevisionId.value : null;
  if (!preserveSelection) {
    selectedRevisionId.value = null;
    selectedTimelineData.value = null;
    selectedNodeMetaSnapshot.value = null;
    selectedRevisionPage.value = null;
  }

  try {
    const environments = await cplaneFetch(`/api/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/environments` as const);

    allEnvironments.value = environments;

    const isDefaultMap = new Map<string, boolean>();
    environments.forEach(b => isDefaultMap.set(b.id, b.is_default));

    if (anchorRevision && graphBook.value.length) {
      const rank = graphBook.value.findIndex(node => node.id === anchorRevision);
      if (rank >= 0) graphPage.value = Math.floor(rank / GRAPH_PAGE_SIZE);
    }
    const pageHead = graphBook.value[graphPage.value * GRAPH_PAGE_SIZE];
    const timelinePageResponse = await cplaneFetch(`/api/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}/timelines` as const, {
      query: pageHead
        ? { limit: GRAPH_PAGE_SIZE, anchor_revision_id: pageHead.id }
        : { graph: true, limit: GRAPH_PAGE_SIZE, anchor_revision_id: anchorRevision ?? undefined },
    });
    if (timelinePageResponse.graph_nodes) {
      graphBook.value = timelinePageResponse.graph_nodes;
      graphPage.value = timelinePageResponse.page ?? 0;
    }
    const graphNodes = new Map(graphBook.value.map(node => [node.id, node]));
    const allTimelines = timelinePageResponse.data;
    graphTotalPages.value = Math.max(1, Math.ceil(graphBook.value.length / GRAPH_PAGE_SIZE));
    hasOlderRevisions.value = graphPage.value + 1 < graphTotalPages.value;
    const visibleIds = new Set(allTimelines.map(rev => rev.id));
    newerRevisionIds.value = new Set(allTimelines.filter(rev =>
      graphNodes.get(rev.id)?.child_ids.some(id => !visibleIds.has(id))).map(rev => rev.id));
    timelineDataMap.value = new Map(allTimelines.map(revision => [revision.id, revision]));

    const environmentDraftTimelineMap = new Map<string, string>();
    const environmentDeployedTimelineMap = new Map<string, string>();
    const environmentNames = new Map<string, string>();
    const environmentPreviewMap = new Map<string, boolean>();
    const environmentExists = new Map<string, boolean>();

    environments.forEach((b) => {
      environmentDraftTimelineMap.set(b.id, b.draft_timeline);
      environmentDeployedTimelineMap.set(b.id, b.deployed_timeline);
      environmentNames.set(b.id, b.name);
      environmentPreviewMap.set(b.id, b.is_preview);
      environmentExists.set(b.id, true);
    });

    for (const t of allTimelines) {
      const environmentId = t.environment_id ?? '__deleted__';
      if (environmentNames.has(environmentId)) continue;
      environmentNames.set(environmentId, 'Deleted Environment');
      environmentPreviewMap.set(environmentId, false);
      environmentExists.set(environmentId, false);
    }

    const allRevs = allTimelines.map(rev => {
      const environmentId = rev.environment_id ?? '__deleted__';
      return {
        environmentId,
        environmentName: environmentNames.get(environmentId) || environmentId,
        rev,
      };
    });

    const pageRevisionIds = new Set(allRevs.map(({ rev }) => rev.id));
    const boundaryChildren: string[] = [];

    const Y_SPACING = 120;
    const NODE_GAP = 20;
    const REVISION_PILL_WIDTH = 112;
    const FORK_LANE_GAP = 40;
    const BOUNDARY_STEP = 160;
    const laneColor = (lane: number) => environmentColors[lane % environmentColors.length]!;

    const environmentLabelsForRevision = (revisionId: string) => {
      const labels: string[] = [];
      for (const [id, name] of environmentNames) {
        const isDraft = environmentDraftTimelineMap.get(id) === revisionId;
        const isDeployed = environmentDeployedTimelineMap.get(id) === revisionId;
        if (!isDraft && !isDeployed) continue;
        if (environmentExists.get(id) ?? false) {
          labels.push(isDraft === isDeployed ? name : `${name} (${isDraft ? 'Draft' : 'Deployed'})`);
        }
      }
      return labels;
    };

    const estimatedNodeWidth = (revisionId: string) => {
      const labels = environmentLabelsForRevision(revisionId);
      if (!labels.length) return REVISION_PILL_WIDTH;
      const displayedLabel = labels.length > 1 ? `${labels[0]} +${labels.length - 1}` : labels[0];
      return REVISION_PILL_WIDTH + 30 + (displayedLabel.length * 7);
    };

    const xPosMap = new Map<string, number>();
    const yLaneMap = new Map<string, number>();
    const laneEndMap = new Map<number, number>();

    const revById = new Map(allRevs.map(({ rev }) => [rev.id, rev]));
    // Lanes come from the whole book. Horizontal spacing is local to the page,
    // but still needs a cursor so page-boundary children do not overlap when
    // their parents are not part of the visible page.
    for (const rev of [...allTimelines].reverse()) {
      const parentX = rev.parent_timeline_id ? xPosMap.get(rev.parent_timeline_id) : undefined;
      const parent = rev.parent_timeline_id ? graphNodes.get(rev.parent_timeline_id) : undefined;
      const node = graphNodes.get(rev.id);
      const lane = node?.lane ?? 0;
      const isFork = parent && node && parent.lane !== node.lane;
      const laneStart = laneEndMap.get(lane) ?? 0;
      const parentPosition = parentX === undefined ? laneStart
        : parentX + estimatedNodeWidth(rev.parent_timeline_id!) + NODE_GAP + (isFork ? FORK_LANE_GAP : 0);
      const x = Math.max(laneStart, parentPosition);
      xPosMap.set(rev.id, x);
      yLaneMap.set(rev.id, lane);
      laneEndMap.set(lane, x + estimatedNodeWidth(rev.id) + NODE_GAP);
    }
    const newNodes: Node[] = [];
    const newEdges: Edge[] = [];
    const metaMap = new Map<string, NodeMeta>();

    for (const { rev, environmentName } of allRevs) {
      const x = xPosMap.get(rev.id) ?? 0;
      const lane = yLaneMap.get(rev.id) ?? 0;
      const y = lane * Y_SPACING;

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
          isPreview: environmentPreviewMap.get(id) || false,
          exists,
          isDraft,
          isDeployed,
        });
      }

      const color = laneColor(lane);

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
          continuesNewer: newerRevisionIds.value.has(rev.id),
          forkLabel: (() => {
            const parentRevision = rev.parent_timeline_id ? revById.get(rev.parent_timeline_id) : undefined;
            return parentRevision && graphNodes.get(parentRevision.id)?.lane !== lane
              ? `Fork · ${environmentName}`
              : undefined;
          })(),
          isSelected: false,
        },
      });

      if (rev.parent_timeline_id) {
        if (pageRevisionIds.has(rev.parent_timeline_id)) {
          const edgeId = `${rev.parent_timeline_id}-${rev.id}`;
          newEdges.push({
            id: edgeId,
            source: rev.parent_timeline_id,
            target: rev.id,
            type: 'default',
            style: { ...HISTORY_EDGE_STYLE, stroke: color },
            animated: true,
          });
        } else {
          // Parent is off-page: mark it as a boundary, not an expired revision.
          boundaryChildren.push(rev.id);
        }
      }
    }

    // Preserve endpoint identity so a continuation can jump across any number
    // of pages, without merging unrelated histories into one boundary node.
    const boundaryRows = new Set<string>();
    function addBoundary(id: string, label: string, x: number, lane: number) {
      const boundaryId = `boundary-${id}-${lane}`;
      let y = lane * Y_SPACING;
      if (!newNodes.some(node => node.id === boundaryId)) {
        while (boundaryRows.has(`${x}:${y}`)) y += Y_SPACING;
        boundaryRows.add(`${x}:${y}`);
        newNodes.push({
          id: boundaryId, type: 'dot', position: { x, y },
          data: { label, targetRevisionId: id, timeline: 0, date: '', environmentName: '', color: laneColor(lane), isHead: false,
            environments: [], environmentLabels: [], isSelected: false },
        });
      }
      return boundaryId;
    }
    for (const childId of boundaryChildren) {
      const parentId = revById.get(childId)!.parent_timeline_id!;
      const lane = yLaneMap.get(childId) ?? 0;
      const source = addBoundary(parentId, 'Earlier revision', -BOUNDARY_STEP * 2,
        lane);
      newEdges.push({ id: `${source}-${childId}`, source, target: childId,
        type: 'default', style: { ...HISTORY_EDGE_STYLE, stroke: laneColor(lane) }, animated: true });
    }
    const newerBoundaryX = Math.max(0, ...allTimelines.map(rev =>
      (xPosMap.get(rev.id) ?? 0) + estimatedNodeWidth(rev.id))) + BOUNDARY_STEP;
    for (const rev of allTimelines) {
      const children = graphNodes.get(rev.id)?.child_ids ?? [];
      for (const childId of children.filter(id => !pageRevisionIds.has(id))) {
        const lane = graphNodes.get(childId)?.lane ?? 0;
        const target = addBoundary(childId, 'Newer revision',
          newerBoundaryX,
          lane);
        newEdges.push({ id: `${rev.id}-${target}`, source: rev.id, target,
          type: 'default', style: { ...HISTORY_EDGE_STYLE, stroke: laneColor(lane) }, animated: true });
      }
    }
    // An edge can cross this entire page without either endpoint appearing here.
    // Keep that path visible, with clickable links to its actual endpoints.
    const oldest = allTimelines.at(-1)?.timeline;
    const newest = allTimelines[0]?.timeline;
    if (oldest !== undefined && newest !== undefined) {
      for (const child of graphBook.value) {
        const parent = child.parent_id ? graphNodes.get(child.parent_id) : undefined;
        if (!parent || parent.timeline >= oldest || child.timeline <= newest) continue;
        const source = addBoundary(parent.id, 'Earlier revision', -BOUNDARY_STEP * 2, child.lane);
        const target = addBoundary(child.id, 'Newer revision', newerBoundaryX, child.lane);
        newEdges.push({ id: `${source}-${target}`, source, target, type: 'default',
          style: { ...HISTORY_EDGE_STYLE, stroke: laneColor(child.lane) }, animated: true });
      }
    }
    if (savedRevisionId && selectedNodeMetaSnapshot.value && !metaMap.has(savedRevisionId)) {
      selectedNodeMetaSnapshot.value = {
        ...selectedNodeMetaSnapshot.value,
        environments: environments
          .filter(env => env.draft_timeline === savedRevisionId || env.deployed_timeline === savedRevisionId)
          .map(env => ({ id: env.id, name: env.name, isDefault: env.is_default,
            isPreview: env.is_preview, exists: true,
            isDraft: env.draft_timeline === savedRevisionId,
            isDeployed: env.deployed_timeline === savedRevisionId })),
      };
    }
    nodeMetaMap.value = metaMap;
    nodes.value = newNodes;
    edges.value = newEdges;

    const revisionToSelect = preserveSelection
      ? savedRevisionId
      : (anchorRevision && newNodes.some(node => node.id === anchorRevision) ? anchorRevision
        : currentRevisionId.value && newNodes.some(node => node.id === currentRevisionId.value)
          ? currentRevisionId.value
          : allTimelines[0]?.id);
    if (revisionToSelect && newNodes.some(node => node.id === revisionToSelect)) {
      if (revisionToSelect !== selectedRevisionId.value) {
        selectedRevisionId.value = revisionToSelect;
      }
      selectRevision(revisionToSelect);
      // The selected id can stay the same while a new page replaces the
      // nodes, so the watcher above does not run. Reapply the visual state.
      nodes.value = nodes.value.map(node => ({
        ...node,
        data: { ...node.data, isSelected: node.id === revisionToSelect },
      }));
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

async function olderRevisions() {
  if (!hasOlderRevisions.value) return;
  graphPage.value += 1;
  await loadGraph(true);
  await nextTick();
  await focusSelectedRevision();
}

async function newerRevisions() {
  if (graphPage.value === 0) return;
  graphPage.value -= 1;
  await loadGraph(true);
  await nextTick();
  await focusSelectedRevision();
}

async function returnToSelectedRevision() {
  if (!selectedRevisionId.value || selectedRevisionVisible.value) return;
  await loadGraph(true, false, selectedRevisionId.value);
  await nextTick();
  await focusSelectedRevision();
}

function navigateAndClose(to: string) {
  open.value = false
  navigateTo(to)
}

function environmentUrl(environmentId: string, revisionId?: string) {
  const base = `/${store.organization?.slug}/compute/containers/${store.project?.id}/${environmentId}`
  return revisionId ? `${base}?revision=${encodeURIComponent(revisionId)}` : base
}

watch(open, (isOpen) => {
  if (isOpen) {
    loadGraph(false, true, currentRevisionId.value);
  }
});

function latestRevisions() {
  loadGraph(false, true);
}
</script>

<template>
  <UModal v-model:open="open" title="Environment Graph" :description="`${store.project?.name ?? ''} — revision timeline`" :ui="{ content: modalContentClass }">
    <template #body>
      <div class="relative flex h-[min(560px,calc(100dvh-12rem))] flex-col gap-4">
        <div v-if="loading" class="flex items-center justify-center h-full"><UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" /></div>
        <p v-else-if="error" class="text-sm text-error">{{ error }}</p>
        <div v-else class="flex flex-1 min-h-0 flex-col gap-3 overflow-hidden lg:flex-row lg:gap-0">
          <!-- Graph area -->
          <div class="flex min-w-0 min-h-64 flex-1 flex-col lg:min-h-0">
            <div class="flex items-center justify-between gap-3 pb-2">
              <p class="text-xs text-muted">{{ graphPageLabel }}</p>
              <div class="flex items-center gap-2">
                <UButton :icon="ICONS.refresh" color="neutral" size="xs" @click="latestRevisions">Latest</UButton>
                <UButton :icon="ICONS.chevronLeft" color="neutral" variant="ghost" size="xs" :disabled="!hasOlderRevisions || loading" @click="olderRevisions">Older</UButton>
                <UButton :trailing-icon="ICONS.chevronRight" color="neutral" variant="ghost" size="xs" :disabled="graphPage === 0 || loading" @click="newerRevisions">Newer</UButton>
              </div>
            </div>
            <div class="flex-1 min-h-0 rounded-lg border border-default bg-default/30">
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
                <Controls :show-interactive="false">
                  <template #control-zoom-in>
                    <ControlButton aria-label="Zoom in" title="Zoom in" @click="graph?.zoomIn()">
                      <UIcon name="i-lucide-plus" class="size-4 text-black" aria-hidden="true" />
                    </ControlButton>
                  </template>
                  <template #control-zoom-out>
                    <ControlButton aria-label="Zoom out" title="Zoom out" @click="graph?.zoomOut()">
                      <UIcon name="i-lucide-minus" class="size-4 text-black" aria-hidden="true" />
                    </ControlButton>
                  </template>
                  <template #control-fit-view>
                    <ControlButton aria-label="Fit visible revisions" title="Fit visible revisions" @click="fitVisibleRevisions()">
                      <UIcon name="i-lucide-maximize" class="size-4 text-black" aria-hidden="true" />
                    </ControlButton>
                  </template>
                </Controls>
              </VueFlow>
            </div>
          </div>

          <!-- Sidebar -->
          <div class="h-auto max-h-64 w-full min-w-0 flex-shrink-0 overflow-hidden lg:h-full lg:max-h-none lg:w-auto">
            <div class="flex h-full w-full flex-col overflow-hidden bg-default/80 lg:w-80">
              <template v-if="selectedTimelineData">
                <!-- Header -->
                <div class="px-5 py-4 border-b border-default/60">
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
                <div v-if="selectedRevisionId && !selectedRevisionVisible" class="flex items-center gap-3 border-b border-default/60 bg-elevated/20 px-5 py-3">
                  <p class="min-w-0 flex-1 text-xs text-muted">Selected revision is on page {{ (selectedRevisionPage ?? 0) + 1 }}.</p>
                  <UButton size="xs" variant="soft" color="neutral" @click="returnToSelectedRevision">Show selected</UButton>
                </div>

                <div class="flex-1 overflow-y-auto px-5 py-5 space-y-5">
                  <!-- Environments -->
                  <div v-if="selectedNodeMeta && selectedNodeMeta.environments.length > 0" class="space-y-2">
                    <div class="flex items-center justify-between">
                      <div class="text-xs font-medium text-muted uppercase tracking-wider">Environments</div>
                      <span class="text-[11px] text-muted">{{ selectedNodeMeta.environments.length }} attached</span>
                    </div>
                    <div class="space-y-2">
                      <div
                        v-for="b in selectedNodeMeta.environments"
                        :key="b.id"
                        class="rounded-lg border border-default/40 bg-elevated/20 p-3"
                      >
                        <button
                          type="button"
                          :disabled="!b.exists"
                          class="group min-w-0 flex w-full flex-1 items-center gap-3 text-left disabled:opacity-50"
                          @click="b.exists ? navigateAndClose(environmentUrl(b.id, selectedRevisionId ?? undefined)) : undefined"
                        >
                          <div class="size-8 shrink-0 rounded-md bg-elevated flex items-center justify-center">
                            <UIcon name="i-heroicons:folder" class="size-4 text-muted" />
                          </div>
                          <span class="min-w-0 flex-1">
                            <span class="text-sm capitalize truncate block">{{ b.name }}</span>
                            <span class="flex flex-wrap items-center gap-x-2 gap-y-0.5 mt-1 text-[11px] text-muted">
                              <span>{{ b.isPreview ? 'Preview' : 'Stable' }}</span>
                              <span v-if="b.isDefault">Default</span>
                              <span v-if="b.isDraft" class="text-warning">Draft</span>
                              <span v-if="b.isDeployed">Deployed</span>
                            </span>
                          </span>
                          <UIcon v-if="b.exists" name="i-heroicons:chevron-right" class="size-3.5 text-muted opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />
                        </button>
                        <div v-if="b.exists" class="mt-3 flex flex-col gap-2">
                          <UButton
                            variant="frosted"
                            size="sm"
                            color="neutral"
                            :icon="ICONS.pencil"
                            block
                            class="justify-center"
                            @click="onRenameEnvironment(b.id)"
                          >
                            Rename environment
                          </UButton>
                          <UButton
                            v-if="!b.isDefault"
                            variant="frosted"
                            size="sm"
                            color="error"
                            :icon="ICONS.trash"
                            block
                            class="justify-center"
                            @click="onRemoveEnvironment(b.id)"
                          >
                            Delete environment
                          </UButton>
                        </div>
                      </div>
                    </div>
                  </div>

                  <!-- History-only revision -->
                  <div v-if="selectedNodeMeta && selectedNodeMeta.environments.length === 0" class="rounded-lg border border-default/40 bg-elevated/20 p-3">
                    <div class="text-xs font-medium uppercase tracking-wider text-muted">History source</div>
                    <p class="mt-2 text-sm">Created in <strong class="capitalize">{{ selectedNodeMeta.environmentName }}</strong>.</p>
                    <p class="mt-1 text-xs text-muted">No environment currently points to this revision, but it remains available for creating or repointing an environment.</p>
                  </div>
                  <div v-else-if="!selectedNodeMeta" class="py-12 text-center">
                    <p class="text-sm text-muted/50">No environment metadata is available for this revision.</p>
                  </div>
                </div>

                <!-- Footer -->
                <div class="px-5 pb-5 pt-4 border-t border-default/60 flex flex-col gap-2.5">
                  <UButton
                    variant="frosted"
                    size="sm"
                    color="neutral"
                    :icon="ICONS.plus"
                    block
                    class="justify-center"
                    @click="onCreateEnvironment(selectedRevisionId!)"
                  >
                    Create environment
                  </UButton>
                  <UButton
                    variant="frosted"
                    size="sm"
                    color="neutral"
                    :icon="ICONS.arrowTopRight"
                    block
                    class="justify-center"
                    @click="onRepointEnvironment(selectedRevisionId!)"
                  >
                    Repoint
                  </UButton>
                </div>
              </template>

              <!-- Empty selection -->
              <div v-else class="flex flex-1 flex-col items-center justify-center px-6 text-center">
                <UIcon name="i-lucide-mouse-pointer-click" class="size-5 text-muted/60" aria-hidden="true" />
                <h3 class="mt-3 text-sm font-medium">Select a revision</h3>
                <p class="mt-1 text-xs text-muted">Choose a node to inspect its environments or use it as the source for a new environment.</p>
              </div>
              </div>
            </div>
          </div>

        <DashboardProjectsCreateEnvironmentModal
          v-model:open="createEnvironmentModalOpen"
          :parent-timeline-id="createEnvironmentFromRevisionId"
          :parent-revision="selectedTimelineData ?? undefined"
          @created="onEnvironmentCreated"
        />

        <UModal v-model:open="repointModalOpen" title="Repoint Environment" :ui="{ content: 'max-w-sm' }">
          <template #body>
            <template v-if="!repointEnvironmentId">
              <p class="text-sm text-muted mb-4">Choose an environment to repoint to this revision.</p>
              <div class="border border-default/40 rounded-lg overflow-hidden max-h-48 overflow-y-auto">
                <button
                  v-for="b in allEnvironments"
                  :key="b.id"
                  class="group w-full flex items-center gap-3 px-4 py-2.5 text-left hover:bg-elevated/50 transition-colors border-b border-default/10 last:border-b-0"
                  @click="selectRepointEnvironment(b.id)"
                >
                  <div class="size-7 shrink-0 rounded-md bg-elevated flex items-center justify-center">
                    <UIcon name="i-heroicons:folder" class="size-3.5 text-muted" />
                  </div>
                  <span class="text-sm capitalize flex-1">{{ b.name }}</span>
                  <UIcon name="i-heroicons:chevron-right" class="size-3.5 text-muted opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />
                </button>
                <p v-if="!allEnvironments.length" class="text-sm text-muted px-4 py-3">No environments available.</p>
              </div>
              <div class="flex justify-end gap-3 pt-3">
                <UButton variant="ghost" color="neutral" @click="repointModalOpen = false; repointEnvironmentId = ''">Cancel</UButton>
              </div>
            </template>
            <template v-else>
              <p class="text-sm">Repoint <strong class="capitalize">{{ repointEnvironment?.name }}</strong> to <strong>{{ repointRevisionLabel }}</strong>?</p>
              <div class="mt-4 rounded-lg border border-warning/40 bg-warning/10 p-3 text-sm text-warning">
                This updates both the draft and deployed revision and starts a deployment immediately.
              </div>
              <div class="flex justify-end gap-3 pt-4">
                <UButton variant="ghost" color="neutral" :disabled="repointing" @click="repointEnvironmentId = ''">Back</UButton>
                <UButton color="primary" :icon="ICONS.check" :loading="repointing" @click="onConfirmRepointEnvironment">Repoint and deploy</UButton>
              </div>
            </template>
          </template>
        </UModal>

        <UModal v-model:open="removeModalOpen" title="Remove Environment" :ui="{ content: 'max-w-sm' }">
          <template #body>
            <p class="text-sm">
              Are you sure you want to remove the environment <strong class="capitalize">{{ removeEnvironmentName }}</strong>? Its revisions are preserved and can be repointed to.
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
