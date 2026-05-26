<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

const store = useStore();
const toast = useToast();

const open = defineModel<boolean>('open', { required: true });
const props = defineProps<{
  organizationId: string;
  type: 'stateful' | 'serverless';
}>();
const emit = defineEmits<{ created: [] }>();

type ProjectOption = {
  id: string;
  name: string;
};

const projectId = ref('');
const projectLabel = computed(() =>
  projects.value.find((p) => p.id === projectId.value)?.name
  || store.project?.name
  || 'Select a project'
);
const loading = ref(false);
const error = ref('');

const projects = ref<ProjectOption[]>([]);
const projectsLoading = ref(false);
const isProjectDropdownOpen = ref(false);

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

const name = ref('');
const cpuCores = ref(0.5);
const ramGib = ref(1);
const highAvailability = ref(false);
const readReplicas = ref(2);

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

watch(open, (isOpen) => {
  if (isOpen) {
    fetchProjects();
    projectId.value = store.project?.id || '';
    name.value = '';
    cpuCores.value = 0.5;
    ramGib.value = 1;
    highAvailability.value = false;
    readReplicas.value = 2;
    error.value = '';
  }
});

watch(isProjectDropdownOpen, (open) => {
  if (open) fetchProjects();
});

async function handleCreate() {
  if (!props.organizationId || !projectId.value || !name.value.trim()) return;

  loading.value = true;
  error.value = '';

  try {
    await $fetch(
      `/api/backend/organization/${props.organizationId}/databases/${props.type}`,
      {
        method: 'POST',
        body: {
          name: name.value.trim(),
          project_id: projectId.value,
          cpu: `${cpuCores.value}`,
          ram: `${Math.round(ramGib.value * 1024)}Mi`,
          high_availability: highAvailability.value,
          read_replicas: highAvailability.value ? readReplicas.value : null,
        },
      }
    );

    const label = props.type === 'stateful' ? 'Stateful' : 'Serverless';
    toast.add({ title: `${label} database created`, color: 'success' });
    name.value = '';
    projectId.value = store.project?.id || '';
    open.value = false;
    emit('created');
  } catch (e: unknown) {
    error.value = (e as { data?: { message?: string } })?.data?.message || (e as { message?: string })?.message || 'Failed to create database';
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <UModal v-model:open="open" title="New Database" :description="`Create a new ${type} Postgres database.`">
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
            v-model="name"
            placeholder="e.g. my-database"
            :disabled="loading"
            autofocus
            class="w-full"
          />
        </UFormField>

        <div class="w-full border border-default rounded-lg">
          <div class="px-4 py-3 border-b border-default">
            <p class="text-sm font-semibold">Compute</p>
          </div>
          <div class="p-4 space-y-3">
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
            <p class="text-sm font-semibold">High Availability</p>
          </div>
          <div class="p-4 space-y-3">
            <UCheckbox v-model="highAvailability" label="Enable high availability" :disabled="loading" />

            <template v-if="highAvailability">
              <div class="flex items-center gap-4 px-1">
                <span class="w-24 text-sm text-foreground">Replicas</span>
                <div class="w-28">
                  <UInput v-model.number="readReplicas" type="number" :min="2" :disabled="loading" size="sm" class="w-full" />
                </div>
                <span class="text-xs text-muted">read replicas (min 2)</span>
              </div>
            </template>
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
            :disabled="!name.trim() || !projectId || (highAvailability && (!readReplicas || readReplicas < 2))"
          >
            Create Database
          </UButton>
        </div>
      </form>
    </template>
  </UModal>
</template>
