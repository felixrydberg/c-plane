<script setup lang="ts">
const store = useStore();
const open = defineModel<boolean>('open', { required: true });
const emit = defineEmits<{ created: [] }>();

const name = ref('');
const loading = ref(false);
const error = ref('');

async function handleCreate() {
  if (!name.value.trim() || !store.organization?.id) return;

  loading.value = true;
  error.value = '';

  try {
    const response = await $fetch<any>(`/api/backend/organization/${store.organization.id}/projects`, {
      method: 'POST',
      body: { name: name.value.trim() },
    });

    store.projects = [...store.projects, {
      id: response.id,
      organization_id: response.organization_id,
      name: response.name,
      default_branch_id: response.default_branch_id,
    }];

    name.value = '';
    open.value = false;
    emit('created');
  } catch (e: any) {
    error.value = e?.data?.message || e?.message || 'Failed to create project';
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <UModal v-model:open="open" title="New Project" description="Create a project to organize your databases, containers, and secrets.">
    <template #body>
      <form class="w-full space-y-4" @submit.prevent="handleCreate">
          <UFormField label="Project Name" required>
            <UInput
              v-model="name"
              placeholder="my-awesome-project"
              :disabled="loading"
              autofocus
              class="w-full"
            />
            <p class="pt-1 text-xs text-muted">
              Choose a name that describes what this project does.
            </p>
          </UFormField>

          <p v-if="error" class="text-sm text-red-500">{{ error }}</p>

          <div class="flex justify-end gap-3 pt-2">
            <UButton variant="ghost" color="neutral" :disabled="loading" @click="open = false">
              Cancel
            </UButton>
            <UButton type="submit" :loading="loading" :disabled="!name.trim()">
              Create Project
            </UButton>
          </div>
        </form>
      </template>
    </UModal>
  </template>
