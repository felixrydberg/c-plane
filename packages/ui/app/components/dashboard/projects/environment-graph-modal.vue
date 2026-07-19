<script setup lang="ts">
import { VueFlow } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { Controls } from '@vue-flow/controls'
import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import '@vue-flow/controls/dist/style.css'
import { markRaw, computed } from 'vue'
import DotNode from './environment-graph-dot-node.vue'
import { ICONS } from '~/utils/icons'

const dotNodeType = markRaw(DotNode)

const store = useStore();
const toast = useToast();

const open = defineModel<boolean>('open', { required: true });

const modalContentClass = 'max-w-7xl';

interface EnvironmentRevision {
  id: string
  environment_id: string
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
  environment_id: string | null
  timeline: number
  name: string | null
  parent_timeline_id: string | null
  containers: ResolvedContainer[]
  secrets: ResolvedSecret[]
  created_at: string
}

interface EnvironmentMeta {
  id: string
  name: string
  isDefault: boolean
  exists: boolean
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
const allEnvironments = ref<{ id: string; name: string }[]>([]);

const removeEnvironmentId = ref('');
const removeEnvironmentName = ref('');
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
      environment_id: data.environment_id ?? null,
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

async function onConfirmRemoveEnvironment() {
  if (!store.organization?.id || !store.project?.id || !removeEnvironmentId.value) return;
  try {
    await $fetch(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/environments/${removeEnvironmentId.value}`,
      { method: 'DELETE' }
    );
    toast.add({ title: 'Environment removed', color: 'success' });
    const deletedId = removeEnvironmentId.value;
    removeModalOpen.value = false;
    removeEnvironmentId.value = '';
    removeEnvironmentName.value = '';

    // Remove from environment list
    allEnvironments.value = allEnvironments.value.filter(b => b.id !== deletedId);

    // Update nodes that had this environment pointing at them
    nodes.value = nodes.value.map(n => {
      const nodeEnvironments: EnvironmentMeta[] = n.data.environments || [];
      const filtered = nodeEnvironments.filter((b: EnvironmentMeta) => b.id !== deletedId);
      if (filtered.length === nodeEnvironments.length) return n;
      return {
        ...n,
        data: {
          ...n.data,
          environments: filtered,
          environmentLabels: filtered.filter((b: EnvironmentMeta) => b.exists).map((b: EnvironmentMeta) => b.name),
          isHead: filtered.some((b: EnvironmentMeta) => b.exists),
        },
      };
    });

    // Update nodeMeta cache
    const updatedMeta = new Map(nodeMetaMap.value);
    for (const [id, meta] of updatedMeta) {
      const filtered = meta.environments.filter(b => b.id !== deletedId);
      if (filtered.length !== meta.environments.length) {
        updatedMeta.set(id, { ...meta, environments: filtered });
      }
    }
    nodeMetaMap.value = updatedMeta;

    // Refresh the sidebar data for the currently selected revision
    if (selectedRevisionId.value) {
      selectRevision(selectedRevisionId.value);
    }
  } catch {
    toast.add({ title: 'Failed to remove environment', color: 'error' });
  }
}

function onEnvironmentCreated(environment: { id: string; name: string; timeline: string; is_default: boolean }) {
  // Add environment to local environment list
  allEnvironments.value = [...allEnvironments.value, { id: environment.id, name: environment.name }];

  // Find the node this environment was forked from and update it
  nodes.value = nodes.value.map(n => {
    if (n.id !== createEnvironmentFromRevisionId.value) return n;
    const nodeEnvironments: EnvironmentMeta[] = n.data.environments || [];
    const newEnvironment: EnvironmentMeta = { id: environment.id, name: environment.name, isDefault: environment.is_default, exists: true };
    const updated = [...nodeEnvironments.filter(b => b.id !== environment.id), newEnvironment];
    return {
      ...n,
      data: {
        ...n.data,
        environments: updated,
        environmentLabels: updated.filter(b => b.exists).map(b => b.name),
        isHead: updated.some(b => b.exists),
      },
    };
  });

  // Update nodeMeta cache
  const updatedMeta = new Map(nodeMetaMap.value);
  const meta = updatedMeta.get(createEnvironmentFromRevisionId.value);
  if (meta) {
    const newEnvironment: EnvironmentMeta = { id: environment.id, name: environment.name, isDefault: environment.is_default, exists: true };
    updatedMeta.set(createEnvironmentFromRevisionId.value, {
      ...meta,
      environments: [...meta.environments.filter(b => b.id !== environment.id), newEnvironment],
    });
  }
  nodeMetaMap.value = updatedMeta;

  // Refresh sidebar data
  if (selectedRevisionId.value) {
    selectRevision(selectedRevisionId.value);
  }
}

async function onSelectRepointEnvironment() {
  if (!store.organization?.id || !store.project?.id || !repointEnvironmentId.value) return;
  try {
    await $fetch(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/environments/${repointEnvironmentId.value}`,
      { method: 'PATCH', body: { timeline_id: repointRevisionId.value } }
    );
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
    const environments = await $fetch<{ id: string; name: string; timeline: string; is_default: boolean }[]>(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/environments`
    );

    allEnvironments.value = environments.map(b => ({ id: b.id, name: b.name }));

    const isDefaultMap = new Map<string, boolean>();
    environments.forEach(b => isDefaultMap.set(b.id, b.is_default));

    const allTimelines = await $fetch<EnvironmentRevision[]>(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/timelines`
    );

    const environmentTimelines = new Map<string, EnvironmentRevision[]>();
    for (const t of allTimelines) {
      if (!t.environment_id) continue;
      const list = environmentTimelines.get(t.environment_id) || [];
      list.push(t);
      environmentTimelines.set(t.environment_id, list);
    }

    const environmentHeadTimelineMap = new Map<string, string>();
    const colorMap = new Map<string, string>();
    const environmentNames = new Map<string, string>();
    const environmentExists = new Map<string, boolean>();

    environments.forEach((b, i) => {
      environmentHeadTimelineMap.set(b.id, b.timeline);
      colorMap.set(b.id, environmentColors[i % environmentColors.length]);
      environmentNames.set(b.id, b.name);
      environmentExists.set(b.id, true);
    });

    for (const environmentId of environmentTimelines.keys()) {
      if (environmentNames.has(environmentId)) continue;
      environmentNames.set(environmentId, 'Deleted Environment');
      colorMap.set(environmentId, '#6b7280');
      environmentExists.set(environmentId, false);
      const revs = environmentTimelines.get(environmentId)!;
      const head = revs.reduce((a, b) => a.timeline > b.timeline ? a : b);
      environmentHeadTimelineMap.set(environmentId, head.id);
    }

    const allRevs: { environmentId: string; environmentName: string; rev: EnvironmentRevision }[] = [];
    for (const [environmentId, revs] of environmentTimelines) {
      const name = environmentNames.get(environmentId) || environmentId;
      for (const rev of revs) {
        allRevs.push({ environmentId, environmentName: name, rev });
      }
    }

    const Y_SPACING = 120;
    const X_SPACING = 360;

    const mainEnvironment = environments.find(b => b.name === 'main') || environments[0];
    const mainRevs = allRevs.filter(r => r.environmentId === mainEnvironment.id).map(r => r.rev).sort((a, b) => Date.parse(a.created_at) - Date.parse(b.created_at));

    const childrenMap = new Map<string, EnvironmentRevision[]>();
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

    for (const { rev, environmentName } of allRevs) {
      if (seen.has(rev.id)) continue;
      seen.add(rev.id);

      const x = xPosMap.get(rev.id) || 0;
      const lane = yLaneMap.get(rev.id) || 0;
      const y = lane * Y_SPACING - centerY;

      const environmentLabels: string[] = [];
      const pointingEnvironments: EnvironmentMeta[] = [];
      for (const [id, name] of environmentNames) {
        if (environmentHeadTimelineMap.get(id) !== rev.id) continue;
        const exists = environmentExists.get(id) ?? false;
        if (exists) {
          environmentLabels.push(name);
        }
        pointingEnvironments.push({
          id,
          name: exists ? name : 'Deleted Environment',
          isDefault: isDefaultMap.get(id) || false,
          exists,
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
                        <UIcon v-if="b.exists" name="i-heroicons:chevron-right" class="size-3.5 text-muted opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />
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
                        @click="selectedTimelineData.environment_id ? navigateAndClose(`/${store.organization?.slug}/containers/${store.project?.id}/${selectedTimelineData.environment_id}/${c.container_id}`) : undefined"
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
                v-for="b in allEnvironments"
                :key="b.id"
                class="group w-full flex items-center gap-3 px-4 py-2.5 text-left hover:bg-elevated/50 transition-colors border-b border-default/10 last:border-b-0"
                @click="repointEnvironmentId = b.id; onSelectRepointEnvironment()"
              >
                <div class="size-7 shrink-0 rounded-md bg-elevated flex items-center justify-center">
                  <UIcon name="i-heroicons:folder" class="size-3.5 text-muted" />
                </div>
                <span class="text-sm capitalize flex-1">{{ b.name }}</span>
                <UIcon name="i-heroicons:chevron-right" class="size-3.5 text-muted opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />
              </button>
              <p v-if="allEnvironments.length === 0" class="text-sm text-muted px-4 py-3">No environments available.</p>
            </div>
            <div class="flex justify-end gap-3 pt-3">
              <UButton variant="ghost"color="neutral" @click="repointModalOpen = false; repointEnvironmentId = ''">Cancel</UButton>
            </div>
          </template>
        </UModal>

        <UModal v-model:open="removeModalOpen" title="Remove Environment" :ui="{ content: 'max-w-sm' }">
          <template #body>
            <p class="text-sm">
              Are you sure you want to remove the environment <strong class="capitalize">{{ removeEnvironmentName }}</strong>? Timeline revisions will be preserved and can be repointed to.
            </p>
            <div class="flex justify-end gap-3 pt-4">
              <UButton variant="ghost" color="neutral" @click="removeModalOpen = false">Cancel</UButton>
              <UButton color="error" @click="onConfirmRemoveEnvironment">Remove</UButton>
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
