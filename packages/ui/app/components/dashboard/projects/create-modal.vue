<script setup lang="ts">
import type { Project } from '@cplane/sdk'
import { getErrorMessage } from '~/utils/errors'
import { ICONS } from '~/utils/icons'

const store = useStore();
const open = defineModel<boolean>('open', { required: true });
const emit = defineEmits<{ created: [project: Project] }>();

const name = ref('');
const loading = ref(false);
const error = ref('');

async function handleCreate() {
  if (!name.value.trim() || !store.organization?.id) return;

  loading.value = true;
  error.value = '';

  try {
    const response = await cplaneFetch<Project>(`/api/organization/${store.organization.id as ':organization_id'}/projects` as const, {
      method: 'POST',
      body: { name: name.value.trim() },
    });

    store.projects = [...store.projects, response];

    name.value = '';
    open.value = false;
    emit('created', response);
  } catch (cause: unknown) {
    error.value = getErrorMessage(cause, 'Failed to create project');
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

          <p v-if="error" class="text-sm text-error">{{ error }}</p>

          <div class="flex justify-end gap-3 pt-2">
            <UButton type="button" variant="ghost" color="neutral" :disabled="loading" @click="open = false">
              Cancel
            </UButton>
            <UButton type="submit" :icon="ICONS.plus" color="primary" variant="frosted" :loading="loading" :disabled="!name.trim()">
              Create Project
            </UButton>
          </div>
        </form>
      </template>
    </UModal>
  </template>
