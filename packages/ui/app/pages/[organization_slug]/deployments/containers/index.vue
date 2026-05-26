<script setup lang="ts">
const store = useStore();

if (!store.organization) {
  throw createError('Organization not found');
}

const MAX_VISIBLE_REPLICAS = 3;

type Region = {
  slug: string;
  display_name: string;
};

type ReplicaStatus = 'running' | 'starting' | 'stopped' | 'error';

type ReplicaMetrics = {
  id: string;
  name: string;
  status: ReplicaStatus;
  cpu_usage: number;
  ram_used: number;
  ram_total: number;
  disk: 'stable' | 'warning' | 'critical';
  network_latency: number;
};

type ContainerDeploymentListItem = {
  id: string;
  name: string;
  created_at: string;
  container: {
    port: number;
    replicas: number;
    public: boolean;
    image: string;
  };
  regions: Region[];
  replica_metrics: ReplicaMetrics[];
};

const allDeployments: ContainerDeploymentListItem[] = [
  {
    id: '7f3e1e2a-0001-4b2a-9c1d-000000000001',
    name: 'api-gateway',
    created_at: '2026-03-01T10:00:00Z',
    container: { port: 8080, replicas: 3, public: true, image: 'ghcr.io/org/api-gateway:latest' },
    regions: [{ slug: 'par-01-amd', display_name: 'PAR-01-AMD' }],
    replica_metrics: [
      { id: 'api-0', name: 'api-gateway-0', status: 'running', cpu_usage: 93, ram_used: 6.1, ram_total: 16, disk: 'stable', network_latency: 0.5 },
      { id: 'api-1', name: 'api-gateway-1', status: 'running', cpu_usage: 67, ram_used: 5.0, ram_total: 16, disk: 'stable', network_latency: 0.8 },
      { id: 'api-2', name: 'api-gateway-2', status: 'starting', cpu_usage: 18, ram_used: 1.9, ram_total: 16, disk: 'stable', network_latency: 1.1 },
    ],
  },
  {
    id: '7f3e1e2a-0001-4b2a-9c1d-000000000002',
    name: 'worker-service',
    created_at: '2026-03-05T14:30:00Z',
    container: { port: 3000, replicas: 2, public: false, image: 'ghcr.io/org/worker:v1.4.0' },
    regions: [{ slug: 'ams-01-amd', display_name: 'AMS-01-AMD' }],
    replica_metrics: [
      { id: 'worker-0', name: 'worker-service-0', status: 'running', cpu_usage: 41, ram_used: 3.2, ram_total: 8, disk: 'stable', network_latency: 1.4 },
      { id: 'worker-1', name: 'worker-service-1', status: 'running', cpu_usage: 38, ram_used: 2.9, ram_total: 8, disk: 'stable', network_latency: 1.7 },
    ],
  },
  {
    id: '7f3e1e2a-0001-4b2a-9c1d-000000000003',
    name: 'auth-service',
    created_at: '2026-03-10T09:15:00Z',
    container: { port: 4000, replicas: 1, public: true, image: 'ghcr.io/org/auth:v2.0.1' },
    regions: [
      { slug: 'par-01-amd', display_name: 'PAR-01-AMD' },
      { slug: 'ams-01-amd', display_name: 'AMS-01-AMD' },
    ],
    replica_metrics: [
      { id: 'auth-0', name: 'auth-service-0', status: 'running', cpu_usage: 52, ram_used: 2.1, ram_total: 8, disk: 'stable', network_latency: 0.6 },
    ],
  },
  {
    id: '7f3e1e2a-0001-4b2a-9c1d-000000000004',
    name: 'batch-job',
    created_at: '2026-03-20T16:45:00Z',
    container: { port: 9090, replicas: 5, public: false, image: 'ghcr.io/org/batch:stable' },
    regions: [{ slug: 'nyc-01-amd', display_name: 'NYC-01-AMD' }],
    replica_metrics: [
      { id: 'batch-0', name: 'batch-job-0', status: 'running', cpu_usage: 74, ram_used: 7.8, ram_total: 16, disk: 'stable', network_latency: 2.1 },
      { id: 'batch-1', name: 'batch-job-1', status: 'running', cpu_usage: 69, ram_used: 7.1, ram_total: 16, disk: 'stable', network_latency: 2.4 },
      { id: 'batch-2', name: 'batch-job-2', status: 'running', cpu_usage: 71, ram_used: 7.5, ram_total: 16, disk: 'stable', network_latency: 2.0 },
      { id: 'batch-3', name: 'batch-job-3', status: 'starting', cpu_usage: 88, ram_used: 12.2, ram_total: 16, disk: 'warning', network_latency: 3.8 },
      { id: 'batch-4', name: 'batch-job-4', status: 'error', cpu_usage: 95, ram_used: 14.5, ram_total: 16, disk: 'critical', network_latency: 5.2 },
    ],
  },
];

const allRegions: Region[] = [
  { slug: 'par-01-amd', display_name: 'PAR-01-AMD (Paris)' },
  { slug: 'ams-01-amd', display_name: 'AMS-01-AMD (Amsterdam)' },
  { slug: 'nyc-01-amd', display_name: 'NYC-01-AMD (New York)' },
];

const selectedRegion = ref<string | null>(null);
const createModalOpen = ref(false);
const expandedReplicaLists = ref<Record<string, boolean>>({});

const filteredDeployments = computed(() => {
  if (!selectedRegion.value) {
    return allDeployments;
  }

  return allDeployments.filter((deployment) =>
    deployment.regions.some((region) => region.slug === selectedRegion.value)
  );
});

const regionOptions: Array<{ slug: string | null; display_name: string }> = [
  { slug: null, display_name: 'All Regions' },
  ...allRegions,
];

const replicaStatusDotClass: Record<ReplicaStatus, string> = {
  running: 'bg-green-500',
  starting: 'bg-yellow-400',
  stopped: 'bg-neutral-400',
  error: 'bg-red-500',
};

const replicaStatusBadgeColor: Record<ReplicaStatus, 'success' | 'warning' | 'neutral' | 'error'> = {
  running: 'success',
  starting: 'warning',
  stopped: 'neutral',
  error: 'error',
};

const replicaStatusLabel: Record<ReplicaStatus, string> = {
  running: 'Running',
  starting: 'Starting',
  stopped: 'Stopped',
  error: 'Error',
};

function isShowingAllReplicas(deploymentId: string) {
  return !!expandedReplicaLists.value[deploymentId];
}

function toggleReplicaList(deploymentId: string) {
  expandedReplicaLists.value[deploymentId] = !expandedReplicaLists.value[deploymentId];
}

function visibleReplicas(deployment: ContainerDeploymentListItem) {
  if (isShowingAllReplicas(deployment.id)) {
    return deployment.replica_metrics;
  }

  return deployment.replica_metrics.slice(0, MAX_VISIBLE_REPLICAS);
}

function hasHiddenReplicas(deployment: ContainerDeploymentListItem) {
  return deployment.replica_metrics.length > MAX_VISIBLE_REPLICAS;
}

function formatRegions(regions: Region[]) {
  return regions.map((region) => region.display_name).join(', ');
}

function refreshDeployments() {
  // TODO: re-fetch deployments list
}
</script>

<template>
  <UDashboardPanel id="containers">
    <template #body>
      <UiPageContainer title="Containers" description="Manage container deployments across regions">
        <div class="flex flex-wrap items-center justify-end gap-3">
          <USelect
            v-model="selectedRegion"
            :options="regionOptions"
            value-attribute="slug"
            label-attribute="display_name"
            placeholder="All Regions"
            class="w-52"
          />
          <UButton
            icon="i-heroicons:plus"
            label="New Container"
            @click="createModalOpen = true"
          />
        </div>

        <div v-if="filteredDeployments.length === 0" class="flex flex-col items-center justify-center py-16 gap-3 text-center">
          <UIcon name="i-heroicons:cube" class="size-10 text-muted" />
          <p class="text-muted">No container deployments found for the selected region.</p>
          <UButton variant="soft" label="Clear filter" @click="selectedRegion = null" />
        </div>

        <div v-else class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
          <UCard
            v-for="deployment in filteredDeployments"
            :key="deployment.id"
            class="flex flex-col gap-3"
            :ui="{ body: 'flex flex-col gap-3' }"
          >
            <div class="flex items-start justify-between gap-2">
              <h3 class="font-semibold text-base truncate">
                {{ deployment.name }}
              </h3>
              <UBadge
                :color="deployment.container.public ? 'success' : 'neutral'"
                variant="soft"
                :label="deployment.container.public ? 'Public' : 'Private'"
                class="shrink-0"
              />
            </div>

            <div class="text-sm text-muted">
              {{ deployment.container.image }}
            </div>

            <div class="flex flex-wrap items-center justify-between gap-2 text-xs text-muted">
              <div class="flex items-center gap-2 flex-wrap min-w-0">
                <span class="inline-flex items-center gap-1 whitespace-nowrap">
                  <UIcon name="i-heroicons:globe-alt" class="size-3.5" />
                  {{ formatRegions(deployment.regions) }}
                </span>
                <span class="text-default/30">|</span>
                <span class="whitespace-nowrap">{{ deployment.container.replicas }} replica{{ deployment.container.replicas !== 1 ? 's' : '' }}</span>
              </div>
              <span class="whitespace-nowrap">Created <NuxtTime :datetime="deployment.created_at" relative /></span>
            </div>

            <div class="pt-2 border-t border-default flex flex-col gap-2">
              <div class="flex items-center justify-between gap-2">
                <h4 class="text-xs font-semibold uppercase tracking-wide text-muted">
                  Replica Usage
                </h4>
                <UButton
                  v-if="hasHiddenReplicas(deployment)"
                  size="xs"
                  variant="ghost"
                  color="neutral"
                  :label="isShowingAllReplicas(deployment.id) ? 'Show Less' : `View All (${deployment.replica_metrics.length})`"
                  @click="toggleReplicaList(deployment.id)"
                />
              </div>

              <div class="flex flex-col gap-2">
                <div
                  v-for="replica in visibleReplicas(deployment)"
                  :key="replica.id"
                  class="rounded-md border border-default bg-default p-3 flex flex-col gap-3"
                >
                  <div class="flex items-center justify-between gap-3">
                    <UBadge
                      size="sm"
                      variant="soft"
                      :color="replicaStatusBadgeColor[replica.status]"
                      :label="replicaStatusLabel[replica.status]"
                    />
                    <div class="flex justify-between text-xs w-full">
                      <span class="truncate text-center">
                        <span class="font-semibold text-default">{{ replica.cpu_usage }}%</span>
                        CPU
                      </span>
                      <span class="truncate text-center">
                        <span class="font-semibold text-default">{{ replica.ram_used }} / {{ replica.ram_total }}GB</span>
                        RAM
                      </span>
                      <span class="truncate text-center">
                        <span class="font-semibold text-default">{{ replica.network_latency }}ms</span>
                        Bandwidth
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </UCard>
        </div>

        <DeploymentsContainersCreateModal
          v-model:open="createModalOpen"
          :organization-id="store.organization!.id"
          @created="refreshDeployments"
        />
      </UiPageContainer>
    </template>
  </UDashboardPanel>
</template>
