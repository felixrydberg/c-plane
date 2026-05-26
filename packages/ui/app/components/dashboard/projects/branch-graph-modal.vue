<script setup lang="ts">
import { VueFlow } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { Controls } from '@vue-flow/controls'
import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import '@vue-flow/controls/dist/style.css'
import { markRaw } from 'vue'
import DotNode from './branch-graph-dot-node.vue'

const dotNodeType = markRaw(DotNode)

const store = useStore();
const toast = useToast();

const open = defineModel<boolean>('open', { required: true });

interface BranchRevision {
  id: string
  branch_id: string
  timeline: number
  name: string | null
  parent_timeline_id: string | null
  created_at: string
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
    removeModalOpen.value = false;
    removeBranchId.value = '';
    removeBranchName.value = '';
    refresh();
  } catch {
    toast.add({ title: 'Failed to remove branch', color: 'error' });
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
    repointModalOpen.value = false;
    repointBranchId.value = '';
    refresh();
  } catch {
    toast.add({ title: 'Failed to repoint branch', color: 'error' });
  }
}

const branchColors = ['#3b82f6', '#22c55e', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4'];

async function loadGraph() {
  if (!store.organization?.id || !store.project?.id) return;
  loading.value = true;
  error.value = '';

  try {
    const branches = await $fetch<{ id: string; name: string; timeline: string; is_default: boolean }[]>(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/branches`
    );

    allBranches.value = branches.map(b => ({ id: b.id, name: b.name }));

    const isDefaultMap = new Map<string, boolean>();
    branches.forEach(b => isDefaultMap.set(b.id, b.is_default));

    const allRevs: { branchId: string; branchName: string; rev: BranchRevision }[] = [];
    for (const branch of branches) {
      const revs = await $fetch<BranchRevision[]>(
        `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/timelines`,
        { query: { branch_id: branch.id } }
      );
      for (const rev of revs) {
        allRevs.push({ branchId: branch.id, branchName: branch.name, rev });
      }
    }

    const branchHeadTimelineMap = new Map<string, string>();
    for (const branch of branches) {
      branchHeadTimelineMap.set(branch.id, branch.timeline);
    }

    const colorMap = new Map<string, string>();
    branches.forEach((b, i) => colorMap.set(b.id, branchColors[i % branchColors.length]));

    const Y_SPACING = 70;
    const X_SPACING = 100;

    // Find the main branch
    const mainBranch = branches.find(b => b.name === 'main') || branches[0];
    const mainRevs = allRevs.filter(r => r.branchId === mainBranch.id).map(r => r.rev).sort((a, b) => a.timeline - b.timeline);

    // Build parent-child map
    const childrenMap = new Map<string, BranchRevision[]>();
    for (const { rev } of allRevs) {
      if (rev.parent_timeline_id) {
        const list = childrenMap.get(rev.parent_timeline_id) || [];
        list.push(rev);
        childrenMap.set(rev.parent_timeline_id, list);
      }
    }

    const lanes = new Map<string, number>();
    const yMap = new Map<string, number>();

    // Main branch gets lane 0
    mainRevs.forEach((r, i) => {
      lanes.set(r.id, 0);
      yMap.set(r.id, -(i * Y_SPACING));
    });

    // Assign lanes for forked branches
    let nextLane = 1;
    for (const mainRev of mainRevs) {
      const forked = childrenMap.get(mainRev.id)?.filter(c => !mainRevs.some(mr => mr.id === c.id)) || [];
      for (const f of forked) {
        const lane = nextLane++;
        // Walk this fork's chain
        let current = f;
        let depth = 0;
        while (current) {
          lanes.set(current.id, lane);
          yMap.set(current.id, yMap.get(mainRev.id)! - ((depth + 1) * Y_SPACING));
          depth++;
          const next = childrenMap.get(current.id)?.[0];
          current = next || null;
        }
      }
    }

    const newNodes: any[] = [];
    const newEdges: any[] = [];
    const processedEdges = new Set<string>();

    const maxLane = Math.max(...lanes.values(), 0);
    const centerOffset = (maxLane * X_SPACING) / 2;

    const seen = new Set<string>();
    for (const { rev } of allRevs) {
      if (seen.has(rev.id)) continue;
      seen.add(rev.id);

      const lane = lanes.get(rev.id) || 0;
      const x = lane * X_SPACING - centerOffset;
      const y = yMap.get(rev.id) || 0;

      const pointingBranches = branches.filter(b => branchHeadTimelineMap.get(b.id) === rev.id);
      const branchLabels = pointingBranches.map(b => b.name);

      newNodes.push({
        id: rev.id,
        type: 'dot',
        position: { x, y },
        data: {
          label: rev.name || `Revision ${rev.timeline}`,
          timeline: rev.timeline,
          date: new Date(rev.created_at).toLocaleString(),
          color: colorMap.get(rev.branch_id) || '#888',
          isHead: branchHeadTimelineMap.get(rev.branch_id) === rev.id,
          isDefault: isDefaultMap.get(rev.branch_id) || false,
          branchName: pointingBranches[0]?.name || rev.branch_id,
          branchId: rev.branch_id,
          branchLabels,
          onCreateBranch,
          onRepointBranch,
          onRemoveBranch,
        },
      });

      if (rev.parent_timeline_id) {
        const edgeId = `${rev.parent_timeline_id}-${rev.id}`;
        if (!processedEdges.has(edgeId)) {
          const parentLane = lanes.get(rev.parent_timeline_id) || 0;
          const isFork = parentLane !== lane;
          newEdges.push({
            id: edgeId,
            source: rev.parent_timeline_id,
            target: rev.id,
            type: isFork ? 'default' : 'straight',
            style: {
              stroke: '#94a3b8',
              strokeWidth: 1.5,
            },
            animated: false,
          });
          processedEdges.add(edgeId);
        }
      }
    }

    nodes.value = newNodes;
    edges.value = newEdges;
  } catch {
    error.value = 'Failed to load graph';
  } finally {
    loading.value = false;
  }
}

function refresh() {
  loadGraph();
}

watch(open, (isOpen) => {
  if (isOpen) loadGraph();
});
</script>

<template>
  <UModal v-model:open="open" title="Branch Graph" :description="`${store.project?.name ?? ''} — revision timeline`" :ui="{ content: 'max-w-7xl' }">
    <template #body>
      <div class="flex flex-col gap-4 relative">
        <div v-if="loading" class="py-8 text-center"><UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" /></div>
        <p v-else-if="error" class="text-sm text-error">{{ error }}</p>
        <div v-else class="w-full rounded-lg border border-default bg-default/30" style="height: 550px">
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
          >
            <Background :gap="20" :size="1" />
            <Controls />
          </VueFlow>
        </div>

        <div class="flex justify-end pt-2">
          <UButton variant="ghost" color="neutral" @click="open = false">Close</UButton>
        </div>

        <DashboardProjectsCreateBranchModal
          v-model:open="createBranchModalOpen"
          :parent-timeline-id="createBranchFromRevisionId"
          @created="refresh"
        />

        <UModal v-model:open="repointModalOpen" title="Repoint Branch" :ui="{ content: 'max-w-sm' }">
          <template #body>
              <div class="flex flex-col gap-0.5 max-h-48 overflow-y-auto">
                <button
                  v-for="b in allBranches"
                  :key="b.id"
                  class="w-full text-left px-3 py-1.5 text-sm rounded-md hover:bg-default/50 flex items-center gap-2 transition-colors"
                  :disabled="repointBranchId === b.id"
                  @click="repointBranchId = b.id; onSelectRepointBranch()"
                >
                  <UIcon
                    :name="repointBranchId === b.id ? 'i-heroicons:arrow-path' : 'i-heroicons:arrow-right-circle'"
                    class="size-4"
                  />
                  {{ b.name }}
                </button>
                <p v-if="allBranches.length === 0" class="text-sm text-muted px-3 py-2">No branches available.</p>
              </div>
              <div class="flex justify-end pt-1">
                <UButton variant="ghost" color="neutral" size="xs" @click="repointModalOpen = false; repointBranchId = ''">Cancel</UButton>
              </div>
            </template>
        </UModal>

        <UModal v-model:open="removeModalOpen" title="Remove Branch" :ui="{ content: 'max-w-sm' }">
          <template #body>
            <p class="text-sm">
              Are you sure you want to remove the branch <strong class="capitalize">{{ removeBranchName }}</strong>? This will delete all its timeline revisions.
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
