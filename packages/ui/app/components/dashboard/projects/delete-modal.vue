<script setup lang="ts">
const store = useStore();
const toast = useToast();

const open = defineModel<boolean>('open', { required: true });
const emit = defineEmits<{ deleted: [] }>();

const loading = ref(false);
const error = ref('');

const projectName = computed(() => store.project?.name ?? '');

async function handleDelete() {
  if (!store.organization?.id || !store.project?.id) return;
  loading.value = true;
  error.value = '';
  try {
    await $fetch(`/api/cplane/organization/${store.organization.id as ':organization_id'}/projects/${store.project.id as ':project_id'}` as const, { method: 'DELETE' });
    store.project = null;
    store.environment = null;
    store.environments = [];
    toast.add({ title: 'Project deleted', color: 'success' });
    open.value = false;
    emit('deleted');
  } catch (e: unknown) {
    error.value = (e as { data?: { message?: string } })?.data?.message || 'Failed to delete project';
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <UModal v-model:open="open">
    <template #body>
      <div class="flex flex-col gap-4">
        <div class="text-center space-y-1">
          <h2 class="text-lg font-semibold">Delete Project</h2>
          <p class="text-sm text-muted">
            Are you sure you want to delete <strong>{{ projectName }}</strong>? This action cannot be undone.
          </p>
        </div>

        <p v-if="error" class="text-sm text-red-500">{{ error }}</p>

        <div class="flex justify-end gap-3 pt-2">
          <UButton variant="ghost" color="neutral" :disabled="loading" @click="open = false">Cancel</UButton>
          <UButton color="error" :loading="loading" @click="handleDelete">Delete</UButton>
        </div>
      </div>
    </template>
  </UModal>
</template>
