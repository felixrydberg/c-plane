<script setup lang="ts">
import { VueFlow } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { Controls } from '@vue-flow/controls'
import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import '@vue-flow/controls/dist/style.css'
import { markRaw, computed } from 'vue'
import DotNode from './branch-graph-dot-node.vue'
import { ICONS } from '~/utils/icons'

const dotNodeType = markRaw(DotNode)

const store = useStore();
const toast = useToast();

const open = defineModel<boolean>('open', { required: true });

const modalContentClass = 'max-w-7xl';

interface BranchRevision {
  id: string
  branch_id: string
  timeline: number
  name: string | null
  parent_timeline_id: string | null
  created_at: string
}

interface ResolvedContainer {
  container_id: string
  container_name: string
  version_id: string
  version: number
  image: string
}

interface ResolvedSecret {
  secret_id: string
  secret_name: string
  version_id: string
  version: number
}

interface ResolvedTimeline {
  id: string
  branch_id: string | null
  timeline: number
  name: string | null
  parent_timeline_id: string | null
  containers: ResolvedContainer[]
  secrets: ResolvedSecret[]
  created_at: string
}

interface BranchMeta {
  id: string
  name: string
  isDefault: boolean
  exists: boolean
}

interface NodeMeta {
  branchName: string
  color: string
  branches: BranchMeta[]
}

const loading = ref(false);
const error = ref('');
const nodes = ref<any[]>([]);
const edges = ref<any[]>([]);

const createBranchModalOpen = ref(false);
const createBranchFromRevisionId = ref('');
const repointModalOpen = ref(false);
const repointRevisionId = ref('');
const repointBranchId = ref('');
const allBranches = ref<{ id: string; name: string }[]>([]);

const removeBranchId = ref('');
const removeBranchName = ref('');
const removeModalOpen = ref(false);

// --- Detail panel state ---
const selectedRevisionId = ref<string | null>(null);
const selectedTimelineData = ref<ResolvedTimeline | null>(null);
const detailLoading = ref(false);
const detailError = ref('');

const nodeMetaMap = ref<Map<string, NodeMeta>>(new Map());

const selectedNodeMeta = computed<NodeMeta | null>(() => {
  if (!selectedRevisionId.value) return null;
  return nodeMetaMap.value.get(selectedRevisionId.value) ?? null;
});

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
    const data = await $fetch<any>(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/timelines/${revisionId}`
    );
    selectedTimelineData.value = {
      id: data.id,
      branch_id: data.branch_id ?? null,
      timeline: data.timeline,
      name: data.name ?? null,
      parent_timeline_id: data.parent_timeline_id ?? null,
      containers: data.containers || [],
      secrets: data.secrets || [],
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

// --- Branch management (called from sidebar) ---
function onCreateBranch(revisionId: string) {
  createBranchFromRevisionId.value = revisionId;
  createBranchModalOpen.value = true;
}

function onRepointBranch(revisionId: string) {
  repointRevisionId.value = revisionId;
  repointModalOpen.value = true;
}

function onRemoveBranch(branchId: string) {
  const branch = allBranches.value.find(b => b.id === branchId);
  removeBranchId.value = branchId;
  removeBranchName.value = branch?.name ?? branchId;
  removeModalOpen.value = true;
}

async function onConfirmRemoveBranch() {
  if (!store.organization?.id || !store.project?.id || !removeBranchId.value) return;
  try {
    await $fetch(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/branches/${removeBranchId.value}`,
      { method: 'DELETE' }
    );
    toast.add({ title: 'Branch removed', color: 'success' });
    const deletedId = removeBranchId.value;
    removeModalOpen.value = false;
    removeBranchId.value = '';
    removeBranchName.value = '';

    // Remove from branch list
    allBranches.value = allBranches.value.filter(b => b.id !== deletedId);

    // Update nodes that had this branch pointing at them
    nodes.value = nodes.value.map(n => {
      const nodeBranches: BranchMeta[] = n.data.branches || [];
      const filtered = nodeBranches.filter((b: BranchMeta) => b.id !== deletedId);
      if (filtered.length === nodeBranches.length) return n;
      return {
        ...n,
        data: {
          ...n.data,
          branches: filtered,
          branchLabels: filtered.filter((b: BranchMeta) => b.exists).map((b: BranchMeta) => b.name),
          isHead: filtered.some((b: BranchMeta) => b.exists),
        },
      };
    });

    // Update nodeMeta cache
    const updatedMeta = new Map(nodeMetaMap.value);
    for (const [id, meta] of updatedMeta) {
      const filtered = meta.branches.filter(b => b.id !== deletedId);
      if (filtered.length !== meta.branches.length) {
        updatedMeta.set(id, { ...meta, branches: filtered });
      }
    }
    nodeMetaMap.value = updatedMeta;

    // Refresh the sidebar data for the currently selected revision
    if (selectedRevisionId.value) {
      selectRevision(selectedRevisionId.value);
    }
  } catch {
    toast.add({ title: 'Failed to remove branch', color: 'error' });
  }
}

function onBranchCreated(branch: { id: string; name: string; timeline: string; is_default: boolean }) {
  // Add branch to local branch list
  allBranches.value = [...allBranches.value, { id: branch.id, name: branch.name }];

  // Find the node this branch was forked from and update it
  nodes.value = nodes.value.map(n => {
    if (n.id !== createBranchFromRevisionId.value) return n;
    const nodeBranches: BranchMeta[] = n.data.branches || [];
    const newBranch: BranchMeta = { id: branch.id, name: branch.name, isDefault: branch.is_default, exists: true };
    const updated = [...nodeBranches.filter(b => b.id !== branch.id), newBranch];
    return {
      ...n,
      data: {
        ...n.data,
        branches: updated,
        branchLabels: updated.filter(b => b.exists).map(b => b.name),
        isHead: updated.some(b => b.exists),
      },
    };
  });

  // Update nodeMeta cache
  const updatedMeta = new Map(nodeMetaMap.value);
  const meta = updatedMeta.get(createBranchFromRevisionId.value);
  if (meta) {
    const newBranch: BranchMeta = { id: branch.id, name: branch.name, isDefault: branch.is_default, exists: true };
    updatedMeta.set(createBranchFromRevisionId.value, {
      ...meta,
      branches: [...meta.branches.filter(b => b.id !== branch.id), newBranch],
    });
  }
  nodeMetaMap.value = updatedMeta;

  // Refresh sidebar data
  if (selectedRevisionId.value) {
    selectRevision(selectedRevisionId.value);
  }
}

async function onSelectRepointBranch() {
  if (!store.organization?.id || !store.project?.id || !repointBranchId.value) return;
  try {
    await $fetch(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/branches/${repointBranchId.value}`,
      { method: 'PATCH', body: { timeline_id: repointRevisionId.value } }
    );
    toast.add({ title: 'Branch repointed', color: 'success' });
    store.refreshKey++;
    repointModalOpen.value = false;
    repointBranchId.value = '';
    refresh();
  } catch {
    toast.add({ title: 'Failed to repoint branch', color: 'error' });
  }
}

const branchColors = ['#3b82f6', '#22c55e', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4'];

async function loadGraph(preserveSelection = false) {
  if (!store.organization?.id || !store.project?.id) return;
  loading.value = true;
  error.value = '';

  // Preserve selection for refreshes triggered by sub-actions (branch create, repoint, delete)
  const savedRevisionId = preserveSelection ? selectedRevisionId.value : null;
  if (!preserveSelection) {
    selectedRevisionId.value = null;
    selectedTimelineData.value = null;
    detailError.value = '';
  }

  try {
    const branches = await $fetch<{ id: string; name: string; timeline: string; is_default: boolean }[]>(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/branches`
    );

    allBranches.value = branches.map(b => ({ id: b.id, name: b.name }));

    const isDefaultMap = new Map<string, boolean>();
    branches.forEach(b => isDefaultMap.set(b.id, b.is_default));

    const allTimelines = await $fetch<BranchRevision[]>(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/timelines`
    );

    const branchTimelines = new Map<string, BranchRevision[]>();
    for (const t of allTimelines) {
      if (!t.branch_id) continue;
      const list = branchTimelines.get(t.branch_id) || [];
      list.push(t);
      branchTimelines.set(t.branch_id, list);
    }

    const branchHeadTimelineMap = new Map<string, string>();
    const colorMap = new Map<string, string>();
    const branchNames = new Map<string, string>();
    const branchExists = new Map<string, boolean>();

    branches.forEach((b, i) => {
      branchHeadTimelineMap.set(b.id, b.timeline);
      colorMap.set(b.id, branchColors[i % branchColors.length]);
      branchNames.set(b.id, b.name);
      branchExists.set(b.id, true);
    });

    for (const branchId of branchTimelines.keys()) {
      if (branchNames.has(branchId)) continue;
      branchNames.set(branchId, 'Deleted Branch');
      colorMap.set(branchId, '#6b7280');
      branchExists.set(branchId, false);
      const revs = branchTimelines.get(branchId)!;
      const head = revs.reduce((a, b) => a.timeline > b.timeline ? a : b);
      branchHeadTimelineMap.set(branchId, head.id);
    }

    const allRevs: { branchId: string; branchName: string; rev: BranchRevision }[] = [];
    for (const [branchId, revs] of branchTimelines) {
      const name = branchNames.get(branchId) || branchId;
      for (const rev of revs) {
        allRevs.push({ branchId, branchName: name, rev });
      }
    }

    const Y_SPACING = 120;
    const X_SPACING = 360;

    const mainBranch = branches.find(b => b.name === 'main') || branches[0];
    const mainRevs = allRevs.filter(r => r.branchId === mainBranch.id).map(r => r.rev).sort((a, b) => a.timeline - b.timeline);

    const childrenMap = new Map<string, BranchRevision[]>();
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
      let useDown = true;
      for (const f of forked) {
        const lane = useDown ? nextDownLane++ : nextUpLane--;
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
        useDown = !useDown;
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

    for (const { rev, branchName } of allRevs) {
      if (seen.has(rev.id)) continue;
      seen.add(rev.id);

      const x = xPosMap.get(rev.id) || 0;
      const lane = yLaneMap.get(rev.id) || 0;
      const y = lane * Y_SPACING - centerY;

      const branchLabels: string[] = [];
      const pointingBranches: BranchMeta[] = [];
      for (const [id, name] of branchNames) {
        if (branchHeadTimelineMap.get(id) !== rev.id) continue;
        const exists = branchExists.get(id) ?? false;
        if (exists) {
          branchLabels.push(name);
        }
        pointingBranches.push({
          id,
          name: exists ? name : 'Deleted Branch',
          isDefault: isDefaultMap.get(id) || false,
          exists,
        });
      }

      const color = colorMap.get(rev.branch_id) || '#888';

      metaMap.set(rev.id, { branchName, color, branches: pointingBranches });

      newNodes.push({
        id: rev.id,
        type: 'dot',
        position: { x, y },
        data: {
          label: rev.name || `Revision ${rev.timeline}`,
          branchName,
          timeline: rev.timeline,
          date: new Date(rev.created_at).toLocaleString(),
          color,
          isHead: pointingBranches.some(b => b.exists),
          branches: pointingBranches,
          branchLabels,
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

    // Re-select the preserved revision after graph reload
    if (savedRevisionId) {
      if (savedRevisionId !== selectedRevisionId.value) {
        selectedRevisionId.value = savedRevisionId;
      }
      selectRevision(savedRevisionId);
    }
  } catch {
    error.value = 'Failed to load graph';
  } finally {
    loading.value = false;
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
  <UModal v-model:open="open" title="Branch Graph" :description="`${store.project?.name ?? ''} — revision timeline`" :ui="{ content: modalContentClass }">
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
              fit-view-on-init
              :nodes-draggable="false"
              :nodes-connectable="false"
              :edges-updatable="false"
              :node-types="{ dot: dotNodeType }"
              @node-click="onNodeClick"
            >
              <Background :gap="20" :size="1" />
              <Controls />
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
                  <!-- Branches -->
                  <div v-if="selectedNodeMeta && selectedNodeMeta.branches.length > 0">
                    <div class="text-xs font-medium text-muted mb-1.5 uppercase tracking-wider">Branches</div>
                    <div class="border border-default/40 rounded-lg overflow-hidden">
                      <button
                        v-for="b in selectedNodeMeta.branches"
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
                        <UIcon v-if="b.exists" name="i-heroicons:chevron-right" class="size-3.5 text-muted opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />
                        <UButton
                          v-if="!b.isDefault && b.exists"
                          variant="solid"
                          size="xs"
                          color="error"
                          :icon="ICONS.trash"
                          @click.prevent.stop="onRemoveBranch(b.id)"
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
                        :disabled="!selectedTimelineData.branch_id"
                        class="group w-full text-left flex items-center gap-3 px-4 py-2.5 hover:bg-elevated/50 transition-colors border-b border-default/10 last:border-b-0 disabled:opacity-50"
                        @click="selectedTimelineData.branch_id ? navigateAndClose(`/${store.organization?.slug}/containers/${store.project?.id}/${selectedTimelineData.branch_id}/${c.container_id}`) : undefined"
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
                    v-if="selectedTimelineData.containers.length === 0 && selectedTimelineData.secrets.length === 0 && (!selectedNodeMeta || selectedNodeMeta.branches.length === 0)"
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
                    @click="onCreateBranch(selectedRevisionId!)"
                  >
                    Create branch
                  </UButton>
                  <UButton
                    variant="solid"
                    size="sm"
                    color="neutral"
                    :icon="ICONS.arrowTopRight"
                    block
                    @click="onRepointBranch(selectedRevisionId!)"
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

        <DashboardProjectsCreateBranchModal
          v-model:open="createBranchModalOpen"
          :parent-timeline-id="createBranchFromRevisionId"
          @created="onBranchCreated"
        />

        <UModal v-model:open="repointModalOpen" title="Repoint Branch" :ui="{ content: 'max-w-sm' }">
          <template #body>
            <p class="text-sm text-muted mb-4">Choose a branch to repoint to this revision.</p>
            <div class="border border-default/40 rounded-lg overflow-hidden max-h-48 overflow-y-auto">
              <button
                v-for="b in allBranches"
                :key="b.id"
                class="group w-full flex items-center gap-3 px-4 py-2.5 text-left hover:bg-elevated/50 transition-colors border-b border-default/10 last:border-b-0"
                @click="repointBranchId = b.id; onSelectRepointBranch()"
              >
                <div class="size-7 shrink-0 rounded-md bg-elevated flex items-center justify-center">
                  <UIcon name="i-heroicons:folder" class="size-3.5 text-muted" />
                </div>
                <span class="text-sm capitalize flex-1">{{ b.name }}</span>
                <UIcon name="i-heroicons:chevron-right" class="size-3.5 text-muted opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />
              </button>
              <p v-if="allBranches.length === 0" class="text-sm text-muted px-4 py-3">No branches available.</p>
            </div>
            <div class="flex justify-end gap-3 pt-3">
              <UButton variant="ghost"color="neutral" @click="repointModalOpen = false; repointBranchId = ''">Cancel</UButton>
            </div>
          </template>
        </UModal>

        <UModal v-model:open="removeModalOpen" title="Remove Branch" :ui="{ content: 'max-w-sm' }">
          <template #body>
            <p class="text-sm">
              Are you sure you want to remove the branch <strong class="capitalize">{{ removeBranchName }}</strong>? Timeline revisions will be preserved and can be repointed to.
            </p>
            <div class="flex justify-end gap-3 pt-4">
              <UButton variant="ghost" color="neutral" @click="removeModalOpen = false">Cancel</UButton>
              <UButton color="error" @click="onConfirmRemoveBranch">Remove</UButton>
            </div>
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
