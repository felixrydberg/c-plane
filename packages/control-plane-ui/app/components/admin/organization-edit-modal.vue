<script setup lang="ts">
const open = defineModel<boolean>();

const props = defineProps<{
  organizationId?: string;
  initialName?: string;
}>();

const emits = defineEmits<{
  (e: 'updated'): void;
}>();

const toast = useToast();
const isSaving = ref(false);
const name = ref('');

type OrganizationEditResponse = {
  organization: {
    id: string;
    name: string;
  };
};

const { data, status, refresh } = await useFetch<OrganizationEditResponse>(() => `/api/admin/organizations/${props.organizationId as ':organization_id'}`, {
  immediate: false,
  watch: [() => open.value, () => props.organizationId],
});

watch([() => open.value, () => props.initialName], () => {
  if (open.value && props.initialName) {
    name.value = props.initialName;
  }
}, { immediate: true });

watch([() => open.value, () => data.value?.organization?.name], () => {
  if (open.value && data.value?.organization?.name) {
    name.value = data.value.organization.name;
  }
}, { immediate: true });

const isLoading = computed(() => status.value === 'pending');

const onSave = async () => {
  if (!props.organizationId || !name.value.trim()) {
    return;
  }

  if (isSaving.value) {
    return;
  }

  isSaving.value = true;
  try {
    await $fetch(`/api/admin/organizations/${props.organizationId}/name`, {
      method: 'PATCH',
      body: {
        name: name.value.trim(),
      },
    });

    toast.add({
      title: 'Success',
      description: 'Organization name updated successfully.',
      color: 'success',
    });

    emits('updated');
    await refresh();
    open.value = false;
  } catch (error) {
    toast.add({
      title: 'An Error accured',
      description: error instanceof Error ? error.message : 'Failed to update organization name.',
      color: 'error',
    });
  } finally {
    isSaving.value = false;
  }
};
</script>

<template>
  <UModal v-model:open="open" title="Edit Organization" description="Update organization details">
    <template #body>
      <div class="space-y-4">
        <div v-if="isLoading" class="py-8 text-center">
          <p class="text-sm text-muted">Loading organization...</p>
        </div>

        <div v-else class="space-y-4">
          <UFormField label="Organization name" name="name" required>
            <UInput
              v-model="name"
              class="w-full"
              placeholder="Enter organization name"
              :disabled="isSaving"
            />
          </UFormField>

          <div class="flex gap-2 justify-end">
            <UButton
              variant="soft"
              :disabled="isSaving"
              @click="open = false"
            >
              Cancel
            </UButton>
            <UButton
              :loading="isSaving"
              :disabled="!name.trim()"
              @click="onSave"
            >
              Save
            </UButton>
          </div>
        </div>
      </div>
    </template>
  </UModal>
</template>
