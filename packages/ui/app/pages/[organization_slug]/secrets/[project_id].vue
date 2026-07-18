<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const store = useStore();
const route = useRoute();
const toast = useToast();

const projectId = computed(() => route.params.project_id?.toString() || null);
const projectName = computed(() =>
  projectId.value ? store.projects.find(p => p.id === projectId.value)?.name : null
);

interface Secret {
  id: string
  project_id: string
  organization_id: string
  name: string
  version_count: number
  deleted_at: string | null
  created_at: string
  updated_at: string
}

interface SecretVersion {
  id: string
  secret_id: string
  version: number
  created_at: string
}

const secrets = ref<Secret[]>([]);
const status = ref<'loading' | 'idle' | 'error'>('loading');
const orgId = computed(() => store.organization?.id);

async function fetchSecrets() {
  if (!orgId.value || !projectId.value) {
    secrets.value = [];
    return;
  }
  status.value = 'loading';
  try {
    const res = await $fetch(`/api/backend/organization/${orgId.value}/projects/${projectId.value}/secrets`);
    secrets.value = (res?.data ?? res ?? []) as Secret[];
    status.value = 'idle';
  } catch {
    secrets.value = [];
    status.value = 'error';
  }
}

watch([orgId, projectId], () => fetchSecrets(), { immediate: true });

const createModalOpen = ref(false);
const createState = ref({ name: '', value: '' });
const createLoading = ref(false);
const createError = ref('');

async function handleCreate() {
  if (!orgId.value || !projectId.value) return;
  createLoading.value = true;
  createError.value = '';
  try {
    await $fetch(`/api/backend/organization/${orgId.value}/projects/${projectId.value}/secrets`, {
      method: 'POST',
      body: { name: createState.value.name.trim(), value: createState.value.value },
    });
    toast.add({ title: 'Secret created', color: 'success' });
    createState.value = { name: '', value: '' };
    createModalOpen.value = false;
    fetchSecrets();
  } catch (e: unknown) {
    createError.value = (e as { data?: { message?: string } })?.data?.message || 'Failed to create secret';
  } finally {
    createLoading.value = false;
  }
}

async function handleDelete(secret: Secret) {
  if (!orgId.value || !projectId.value) return;
  try {
    await $fetch(`/api/backend/organization/${orgId.value}/projects/${projectId.value}/secrets/${secret.id}`, {
      method: 'DELETE',
    });
    toast.add({ title: 'Secret deleted', color: 'success' });
    fetchSecrets();
  } catch {
    toast.add({ title: 'Failed to delete secret', color: 'error' });
  }
}

const editModalOpen = ref(false);
const editingSecret = ref<Secret | null>(null);
const editValue = ref('');
const editLoading = ref(false);
const editError = ref('');
const versions = ref<SecretVersion[]>([]);

async function openEdit(secret: Secret) {
  editingSecret.value = secret;
  editValue.value = '';
  versions.value = [];
  editModalOpen.value = true;
  if (!orgId.value || !projectId.value) return;
  try {
    const res = await $fetch(`/api/backend/organization/${orgId.value}/projects/${projectId.value}/secrets/${secret.id}/versions`);
    versions.value = (res?.data ?? res ?? []) as SecretVersion[];
  } catch { /* ignore */ }
}

async function handleUpdate() {
  if (!orgId.value || !projectId.value || !editingSecret.value || !editValue.value) return;
  editLoading.value = true;
  editError.value = '';
  try {
    await $fetch(`/api/backend/organization/${orgId.value}/projects/${projectId.value}/secrets/${editingSecret.value.id}`, {
      method: 'PATCH',
      body: { value: editValue.value },
    });
    toast.add({ title: 'Secret updated', color: 'success' });
    editModalOpen.value = false;
    fetchSecrets();
  } catch (e: unknown) {
    editError.value = (e as { data?: { message?: string } })?.data?.message || 'Failed to update secret';
  } finally {
    editLoading.value = false;
  }
}

</script>

<template>
  <div class="flex w-full max-w-[1500px] flex-col gap-5 mx-auto">
    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <h1 class="text-2xl font-semibold">Secrets</h1>
        <p class="text-muted text-sm mt-1">
          Manage secrets for {{ projectName ?? 'this project' }}.
        </p>
      </div>
      <div class="flex items-center gap-3">
        <UButton
          v-if="projectId"
          :icon="ICONS.plus"
          @click="createModalOpen = true"
        >
          New Secret
        </UButton>
      </div>
    </div>

    <div v-if="status === 'loading'" class="text-center py-8">
      <UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" />
    </div>

    <div v-else-if="projectId && secrets.length > 0" class="rounded-lg border border-dashed border-default bg-transparent divide-y divide-default">
      <div
        v-for="s in secrets"
        :key="s.id"
        class="flex items-center justify-between px-4 py-3 hover:bg-elevated transition-colors"
      >
        <div>
          <p class="text-sm font-medium">{{ s.name }}</p>
          <p class="text-xs text-muted">{{ s.version_count }} version{{ s.version_count !== 1 ? 's' : '' }}</p>
        </div>
        <div class="flex items-center gap-2">
          <UBadge size="sm" variant="soft" color="neutral">{{ s.version_count }} versions</UBadge>
          <UDropdownMenu
            :items="[[
              { label: 'Edit', icon: ICONS.pencil, onSelect: () => openEdit(s) },
              { label: 'Delete', icon: ICONS.trash, color: 'error', onSelect: () => handleDelete(s) },
            ]]"
            :content="{ align: 'end' }"
          >
            <UButton variant="solid" size="xs" color="neutral" :icon="ICONS.general" aria-label="Actions" />
          </UDropdownMenu>
        </div>
      </div>
    </div>

    <div
      v-else-if="projectId && secrets.length === 0 && status === 'idle'"
      class="flex flex-col items-center justify-center py-16 gap-3 text-center border border-dashed border-default rounded-lg"
    >
      <UIcon :name="ICONS.secrets" class="size-10 text-muted" />
      <p class="text-muted">No secrets configured. Add one to get started.</p>
    </div>


    <UModal v-model:open="createModalOpen" title="New Secret" description="Add a new environment variable secret to this project.">
      <template #body>
        <form class="space-y-4" @submit.prevent="handleCreate">
          <UFormField label="Name" required>
            <UInput
              v-model="createState.name"
              placeholder="e.g. DATABASE_URL"
              :disabled="createLoading"
              class="w-full"
            />
          </UFormField>
          <UFormField label="Value" required>
            <UInput
              v-model="createState.value"
              placeholder="e.g. postgres://..."
              type="password"
              :disabled="createLoading"
              class="w-full"
            />
          </UFormField>
          <p v-if="createError" class="text-sm text-error">{{ createError }}</p>
          <div class="flex justify-end gap-3 pt-2">
            <UButton variant="ghost" color="neutral" :disabled="createLoading" @click="createModalOpen = false">Cancel</UButton>
            <UButton type="submit" :loading="createLoading" :disabled="!createState.name.trim() || !createState.value">Create</UButton>
          </div>
        </form>
      </template>
    </UModal>

    <UModal v-model:open="editModalOpen" title="Update Secret" :description="editingSecret ? `Update value for ${editingSecret.name}` : ''">
      <template #body>
        <form class="space-y-4" @submit.prevent="handleUpdate">
          <UFormField label="New Value" required>
            <UInput
              v-model="editValue"
              placeholder="Enter new value"
              type="password"
              :disabled="editLoading"
              class="w-full"
            />
          </UFormField>
          <div v-if="versions.length > 0" class="border border-default rounded-lg">
            <div class="px-3 py-2 border-b border-default">
              <p class="text-xs font-semibold text-muted">VERSION HISTORY</p>
            </div>
            <div class="divide-y divide-default max-h-40 overflow-y-auto">
              <div v-for="v in versions" :key="v.id" class="flex items-center justify-between px-3 py-2 text-sm">
                <span>Version {{ v.version }}</span>
                <NuxtTime :datetime="v.created_at" relative class="text-xs text-muted" />
              </div>
            </div>
          </div>
          <p v-if="editError" class="text-sm text-error">{{ editError }}</p>
          <div class="flex justify-end gap-3 pt-2">
            <UButton variant="ghost" color="neutral" :disabled="editLoading" @click="editModalOpen = false">Cancel</UButton>
            <UButton type="submit" :loading="editLoading" :disabled="!editValue">Update</UButton>
          </div>
        </form>
      </template>
    </UModal>
  </div>
</template>
