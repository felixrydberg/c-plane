<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

const store = useStore();
const toast = useToast();

const open = defineModel<boolean>('open', { required: true });
const props = defineProps<{
  organizationId: string;
}>();
const emit = defineEmits<{ created: [] }>();

type RegionOption = {
  id: string;
  slug: string;
  display_name: string;
};

type ProjectOption = {
  id: string;
  name: string;
};

const projectId = ref('');
const regionId = ref('');
const projectLabel = computed(() =>
  projects.value.find((p) => p.id === projectId.value)?.name
  || store.project?.name
  || 'Select a project'
);
const regionLabel = computed(() =>
  regions.value.find((r) => r.id === regionId.value)?.display_name
  || 'Select a region'
);
const loading = ref(false);
const error = ref('');

const projects = ref<ProjectOption[]>([]);
const projectsLoading = ref(false);
const isProjectDropdownOpen = ref(false);

const regions = ref<RegionOption[]>([]);
const regionsLoading = ref(false);
const isRegionDropdownOpen = ref(false);

const projectMenuItems = computed<DropdownMenuItem[][]>(() => {
  const list: DropdownMenuItem[] = [];

  for (const p of projects.value) {
    list.push({
      label: p.name,
      icon: ICONS.folder,
      onSelect() {
        projectId.value = p.id;
      },
    });
  }

  return [list];
});

const regionMenuItems = computed<DropdownMenuItem[][]>(() => {
  const list: DropdownMenuItem[] = [];

  for (const r of regions.value) {
    list.push({
      label: r.display_name,
      onSelect() {
        regionId.value = r.id;
      },
    });
  }

  return [list];
});

const state = ref({
  name: '',
  image: '',
  port: 80,
  replicas: 1,
  is_public: false,
  health_check_path: '/health',
});

const cpuCores = ref(0.5);
const ramGib = ref(1);

const cpuPresets = [0.25, 0.5, 1, 2, 4, 8, 16, 32];
const ramPresets = [0.25, 0.5, 1, 2, 4, 8, 16, 32, 64, 128, 256];

async function fetchProjects() {
  if (!props.organizationId) return;
  projectsLoading.value = true;
  try {
    const response = await $fetch(`/api/backend/organization/${props.organizationId}/projects`);
    projects.value = (response?.data ?? []).map((p: { id: string; name: string }) => ({ id: p.id, name: p.name }));
  } catch {
    projects.value = [];
  } finally {
    projectsLoading.value = false;
  }
}

async function fetchRegions() {
  if (!props.organizationId) return;
  regionsLoading.value = true;
  try {
    regions.value = await $fetch<RegionOption[]>(`/api/organization/${props.organizationId}/regions`);
  } catch {
    regions.value = [];
  } finally {
    regionsLoading.value = false;
  }
}

watch(open, (isOpen) => {
  if (isOpen) {
    fetchProjects();
    fetchRegions();
    projectId.value = store.project?.id || '';
    regionId.value = '';
  }
});

watch(isProjectDropdownOpen, (open) => {
  if (open) fetchProjects();
});

watch(isRegionDropdownOpen, (open) => {
  if (open) fetchRegions();
});

async function handleCreate() {
  if (!props.organizationId || !projectId.value || !regionId.value || !store.branch?.id) return;

  loading.value = true;
  error.value = '';

  try {
    const resources = {
      cpu: { min: cpuCores.value, max: cpuCores.value },
      memory: {
        min: `${Math.round(ramGib.value * 1024)}Mi`,
        max: `${Math.round(ramGib.value * 1024)}Mi`,
      },
    };

    await $fetch(
      `/api/backend/organization/${props.organizationId}/containers`,
      {
        method: 'POST',
        body: {
          name: state.value.name.trim(),
          image: state.value.image.trim(),
          project_id: projectId.value,
          branch_id: store.branch!.id,
          port: state.value.port,
          replica_count: state.value.replicas,
          public: state.value.is_public,
          health_check: { path: state.value.health_check_path },
          region_id: regionId.value,
          resources,
        },
      }
    );

    toast.add({ title: 'Container created', color: 'success' });
    state.value = {
      name: '',
      image: '',
      port: 80,
      replicas: 1,
      is_public: false,
      health_check_path: '/health',
    };
    cpuCores.value = 0.5;
    ramGib.value = 1;
    projectId.value = store.project?.id || '';
    regionId.value = '';
    open.value = false;
    emit('created');
  } catch (e: unknown) {
    error.value = (e as { data?: { message?: string } })?.data?.message || (e as { message?: string })?.message || 'Failed to create container';
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <UModal v-model:open="open" title="New Container" description="Deploy a new container to your project.">
    <template #body>
      <form class="w-full space-y-4" @submit.prevent="handleCreate">
          <UFormField label="Project" required>
            <UDropdownMenu
              :items="projectMenuItems"
              :content="{ align: 'center', collisionPadding: 12 }"
              :ui="{ content: 'w-(--reka-dropdown-menu-trigger-width)' }"
              @open="isProjectDropdownOpen = true"
              @close="isProjectDropdownOpen = false"
            >
              <UButton
                :label="projectLabel"
                trailing-icon="i-lucide-chevrons-up-down"
                color="neutral"
                variant="soft"
                block
                :disabled="loading || projectsLoading"
                class="data-[state=open]:bg-elevated w-full"
                :ui="{ trailingIcon: 'text-dimmed' }"
              />
            </UDropdownMenu>
          </UFormField>

          <UFormField label="Name" required>
            <UInput
              v-model="state.name"
              placeholder="e.g. api-gateway"
              :disabled="loading"
              autofocus
              class="w-full"
            />
          </UFormField>

          <UFormField label="Image" required>
            <UInput
              v-model="state.image"
              placeholder="e.g. nginx:latest"
              :disabled="loading"
              class="w-full"
            />
          </UFormField>

          <USeparator label="Resource Limits" class="my-2" />

          <div class="w-full border border-default rounded-lg">
            <div class="px-4 py-3 border-b border-default">
              <p class="text-sm font-semibold">Resource Limits</p>
            </div>
            <div class="p-4 space-y-3">
              <div class="flex items-center gap-4 text-xs text-muted px-1">
                <span class="w-24">Resource</span>
                <span class="w-28">Reservation</span>
              </div>
              <div class="flex items-center gap-4 px-1">
                <span class="w-24 text-sm text-foreground">CPU</span>
                <div class="w-28">
                  <USelect v-model="cpuCores" :items="cpuPresets" :disabled="loading" size="sm" class="w-full" />
                </div>
                <span class="text-xs text-muted">cores</span>
              </div>

              <div class="flex items-center gap-4 px-1">
                <span class="w-24 text-sm text-foreground">RAM</span>
                <div class="w-28">
                  <USelect v-model="ramGib" :items="ramPresets" :disabled="loading" size="sm" class="w-full" />
                </div>
                <span class="text-xs text-muted">GiB</span>
              </div>
            </div>
          </div>

          <div class="w-full border border-default rounded-lg">
            <div class="px-4 py-3 border-b border-default">
              <p class="text-sm font-semibold">Deployment Config</p>
            </div>
            <div class="p-4 space-y-3">
              <div class="grid grid-cols-2 gap-4">
                <div class="flex items-center gap-3">
                  <span class="text-sm text-muted w-12">Port</span>
                  <UInput
                    v-model.number="state.port"
                    :min="1"
                    :max="65535"
                    :disabled="loading"
                    class="flex-1"
                  />
                </div>
                <div class="flex items-center gap-3">
                  <span class="text-sm text-muted w-16">Replicas</span>
                  <UInput
                    v-model.number="state.replicas"
                    :min="1"
                    :disabled="loading"
                    class="flex-1"
                  />
                </div>
              </div>

              <div class="flex items-center gap-3">
                <span class="text-sm text-muted w-12">Region</span>
                <UDropdownMenu
                  :items="regionMenuItems"
                  :content="{ align: 'center', collisionPadding: 12 }"
                  :ui="{ content: 'w-(--reka-dropdown-menu-trigger-width)' }"
                  @open="isRegionDropdownOpen = true"
                  @close="isRegionDropdownOpen = false"
                >
                  <UButton
                    :label="regionLabel"
                    trailing-icon="i-lucide-chevrons-up-down"
                    color="neutral"
                    variant="soft"
                    block
                    :disabled="loading || regionsLoading"
                    class="data-[state=open]:bg-elevated"
                    :ui="{ trailingIcon: 'text-dimmed' }"
                  />
                </UDropdownMenu>
              </div>

              <div class="flex items-center gap-3">
                <span class="text-sm text-muted w-12">Health</span>
                <UInput
                  v-model="state.health_check_path"
                  placeholder="/health"
                  :disabled="loading"
                  class="flex-1"
                />
              </div>

              <UCheckbox v-model="state.is_public" label="Public endpoint" :disabled="loading" />
            </div>
          </div>

          <p v-if="error" class="text-sm text-error">{{ error }}</p>

          <div class="flex justify-end gap-3 pt-2">
            <UButton variant="ghost" color="neutral" :disabled="loading" @click="open = false">
              Cancel
            </UButton>
            <UButton
              type="submit"
              :loading="loading"
              :disabled="!state.name.trim() || !state.image.trim() || !regionId || !projectId || !store.branch?.id"
            >
              Create Container
            </UButton>
          </div>
        </form>
      </template>
    </UModal>
  </template>
