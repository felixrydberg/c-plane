<script setup lang="ts">
const toast = useToast();

const open = defineModel<boolean>('open', { required: true });

const props = defineProps<{
  organizationId: string;
  containerId: string;
  containerName: string;
  branchId: string | null;
  currentVersion: {
    image: string;
    public: boolean;
    replica_count: number;
    port: number | null;
    health_check: Record<string, unknown> | null;
  } | null;
}>();

const emit = defineEmits<{ updated: [] }>();

const loading = ref(false);
const error = ref('');

const state = ref({
  name: props.containerName,
  image: props.currentVersion?.image ?? '',
  port: props.currentVersion?.port ?? 80,
  replicas: props.currentVersion?.replica_count ?? 1,
  is_public: props.currentVersion?.public ?? false,
  health_check_path: (props.currentVersion?.health_check as { path?: string } | null)?.path ?? '/health',
});

watch(open, (isOpen) => {
  if (isOpen) {
    state.value = {
      name: props.containerName,
      image: props.currentVersion?.image ?? '',
      port: props.currentVersion?.port ?? 80,
      replicas: props.currentVersion?.replica_count ?? 1,
      is_public: props.currentVersion?.public ?? false,
      health_check_path: (props.currentVersion?.health_check as { path?: string } | null)?.path ?? '/health',
    };
    error.value = '';
  }
});

async function handleUpdate() {
  if (!props.organizationId || !props.containerId) return;

  loading.value = true;
  error.value = '';

  const body: Record<string, unknown> = {};

  const newName = state.value.name.trim();
  const newImage = state.value.image.trim();

  if (newName && newName !== props.containerName) {
    body.name = newName;
  }
  if (newImage && newImage !== (props.currentVersion?.image ?? '')) {
    body.image = newImage;
  }
  if (state.value.port !== (props.currentVersion?.port ?? 80)) {
    body.port = state.value.port;
  }
  if (state.value.replicas !== (props.currentVersion?.replica_count ?? 1)) {
    body.replica_count = state.value.replicas;
  }
  if (state.value.is_public !== (props.currentVersion?.public ?? false)) {
    body.public = state.value.is_public;
  }

  const currentPath = (props.currentVersion?.health_check as { path?: string } | null)?.path ?? '/health';
  if (state.value.health_check_path !== currentPath) {
    body.health_check = { path: state.value.health_check_path };
  }

  if (Object.keys(body).length === 0) {
    open.value = false;
    return;
  }

  try {
    await $fetch(
      `/api/backend/organization/${props.organizationId}/containers/${props.containerId}?branch_id=${props.branchId ?? ''}`,
      {
        method: 'PATCH',
        body,
      }
    );

    toast.add({ title: 'Container updated', color: 'success' });
    open.value = false;
    emit('updated');
  } catch (e: unknown) {
    error.value = (e as { data?: { message?: string } })?.data?.message || (e as { message?: string })?.message || 'Failed to update container';
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <UModal v-model:open="open" title="Edit Container" :description="`Update ${props.containerName} configuration.`">
    <template #body>
      <form class="w-full space-y-4" @submit.prevent="handleUpdate">
          <UFormField label="Name" required>
            <UInput
              v-model="state.name"
              placeholder="e.g. api-gateway"
              :disabled="loading"
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

          <div class="grid grid-cols-2 gap-4">
            <UFormField label="Port">
              <UInput
                v-model.number="state.port"
                type="number"
                :min="1"
                :max="65535"
                :disabled="loading"
                class="w-full"
              />
            </UFormField>

            <UFormField label="Replicas">
              <UInput
                v-model.number="state.replicas"
                type="number"
                :min="1"
                :disabled="loading"
                class="w-full"
              />
            </UFormField>
          </div>

          <UFormField label="Health Check Path">
            <UInput
              v-model="state.health_check_path"
              placeholder="/health"
              :disabled="loading"
              class="w-full"
            />
          </UFormField>

          <UFormField>
            <UCheckbox v-model="state.is_public" label="Public endpoint" :disabled="loading" />
          </UFormField>

          <p v-if="error" class="text-sm text-red-500">{{ error }}</p>

          <div class="flex justify-end gap-3 pt-2">
            <UButton variant="ghost" color="neutral" :disabled="loading" @click="open = false">
              Cancel
            </UButton>
            <UButton
              type="submit"
              :loading="loading"
              :disabled="!state.name.trim() || !state.image.trim()"
            >
              Save Changes
            </UButton>
          </div>
        </form>
      </template>
    </UModal>
  </template>
