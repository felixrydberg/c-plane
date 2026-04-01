<script setup lang="ts">
const open = defineModel<boolean>();

const props = defineProps<{
  organizationId?: string;
}>();

type OrganizationDeleteResponse = {
  organization: {
    id: string;
    name: string;
  };
};

const emits = defineEmits<{
  (e: 'deleted'): void;
}>();

const toast = useToast();
const isDeleting = ref(false);

const { data, status } = await useFetch<OrganizationDeleteResponse>(() => `/api/admin/organizations/${props.organizationId as ':organization_id'}`, {
  immediate: false,
  watch: [() => open.value, () => props.organizationId],
});

const organizationDetails = computed(() => data.value?.organization);
const isLoading = computed(() => status.value === 'pending');

const onDelete = async () => {
  if (!props.organizationId || isDeleting.value) {
    return;
  }

  isDeleting.value = true;
  try {
    await $fetch(`/api/admin/organizations/${props.organizationId}`, {
      method: 'DELETE',
    });

    toast.add({
      title: 'Success',
      description: 'Organization and Polar customer were deleted successfully.',
      color: 'success',
    });

    emits('deleted');
    open.value = false;
  } catch (error) {
    toast.add({
      title: 'An Error accured',
      description: error instanceof Error ? error.message : 'Failed to delete organization.',
      color: 'error',
    });
  } finally {
    isDeleting.value = false;
  }
};
</script>

<template>
  <UModal v-model:open="open" title="Delete Organization" description="This action cannot be undone">
    <template #body>
      <div class="space-y-4">
        <div v-if="isLoading" class="py-8 text-center">
          <p class="text-sm text-muted">Loading organization...</p>
        </div>

        <div v-else class="space-y-4">
          <p class="text-sm">
            Are you sure you want to delete
            <strong>{{ organizationDetails?.name || 'this organization' }}</strong>?
            This action cannot be undone.
          </p>
          <p class="text-sm text-warning">
            This will also delete the linked Polar customer.
          </p>

          <div class="flex gap-2 justify-end">
            <UButton
              variant="soft"
              :disabled="isDeleting"
              @click="open = false"
            >
              Cancel
            </UButton>
            <UButton
              color="error"
              :loading="isDeleting"
              @click="onDelete"
            >
              Delete
            </UButton>
          </div>
        </div>
      </div>
    </template>
  </UModal>
</template>
