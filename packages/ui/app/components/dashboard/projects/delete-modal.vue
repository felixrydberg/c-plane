<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const store = useStore();
const toast = useToast();

const open = defineModel<boolean>('open', { required: true });
const emit = defineEmits<{ deleted: [] }>();

const loading = ref(false);
const error = ref('');
const confirmation = ref('');

const projectName = computed(() => store.project?.name ?? '');
const isConfirmed = computed(() => confirmation.value === projectName.value && projectName.value.length > 0);

watch(open, (isOpen) => {
  if (isOpen) {
    confirmation.value = '';
    error.value = '';
  }
});

async function handleDelete() {
  if (!store.organization?.id || !store.project?.id || !isConfirmed.value) return;
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
  <UModal
    v-model:open="open"
    title="Delete Project"
    description="This action cannot be undone."
    :ui="{ description: 'text-xs' }"
  >
    <template #body>
      <form class="flex flex-col gap-4" @submit.prevent="handleDelete">
        <p class="text-sm text-error">
          All running services and backups for <strong>{{ projectName }}</strong> will be deleted permanently.
        </p>

        <UFormField
          label="Confirm project name"
          :description="`Type ${projectName} to confirm deletion.`"
          :error="confirmation && !isConfirmed ? 'The project name does not match.' : undefined"
          required
        >
          <UInput
            v-model="confirmation"
            :placeholder="projectName"
            :disabled="loading"
            autocomplete="off"
            autofocus
            class="w-full"
          />
        </UFormField>

        <p v-if="error" class="text-sm text-red-500">{{ error }}</p>

        <div class="flex justify-end gap-3 pt-2">
          <UButton variant="ghost" color="neutral" :disabled="loading" @click="open = false">Cancel</UButton>
          <UButton type="submit" :icon="ICONS.trash" color="error" :loading="loading" :disabled="!isConfirmed">Delete</UButton>
        </div>
      </form>
    </template>
  </UModal>
</template>
