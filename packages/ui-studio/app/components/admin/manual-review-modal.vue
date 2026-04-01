<script setup lang="ts">
const open = defineModel<boolean>();

const props = defineProps<{
  verificationId?: number;
  verificationPublicId?: string;
}>();

const emit = defineEmits<{
  (e: "reviewed"): void;
}>();

const NuxtTime = resolveComponent("NuxtTime");
const toast = useToast();

const reviewNotes = ref("");
const isSubmitting = ref(false);

const { data, status, refresh } = await useFetch(() => `/api/admin/manual-reviews/${String(props.verificationId || 0) as ':verification_id'}/assets`, {
  immediate: false,
  watch: [() => open.value, () => props.verificationId],
});

type ManualReviewAsset = NonNullable<typeof data.value>["data"][number];
const assets = computed(() => data.value?.data ?? [] as ManualReviewAsset[]);
const isLoading = computed(() => status.value === "pending");

const approve = async () => {
  if (!props.verificationId) {
    return;
  }

  isSubmitting.value = true;
  try {
    await $fetch(`/api/admin/manual-reviews/${String(props.verificationId) as ':verification_id'}/decision`, {
      method: "POST",
      body: {
        decision: "approve",
        review_notes: reviewNotes.value,
      },
    });

    toast.add({
      title: "Success",
      description: "Manual review approved",
      color: "success",
    });

    open.value = false;
    emit("reviewed");
  } catch {
    toast.add({
      title: "An Error accured",
      description: "Failed to approve manual review",
      color: "error",
    });
  } finally {
    isSubmitting.value = false;
  }
};

const reject = async () => {
  if (!props.verificationId) {
    return;
  }

  isSubmitting.value = true;
  try {
    await $fetch(`/api/admin/manual-reviews/${String(props.verificationId) as ':verification_id'}/decision`, {
      method: "POST",
      body: {
        decision: "reject",
        review_notes: reviewNotes.value,
      },
    });

    toast.add({
      title: "Success",
      description: "Manual review rejected",
      color: "success",
    });

    open.value = false;
    emit("reviewed");
  } catch {
    toast.add({
      title: "An Error accured",
      description: "Failed to reject manual review",
      color: "error",
    });
  } finally {
    isSubmitting.value = false;
  }
};

watch(() => open.value, (isOpen) => {
  if (!isOpen) {
    reviewNotes.value = "";
    return;
  }

  void refresh();
});
</script>

<template>
  <UModal
    v-model:open="open"
    title="Manual Review"
    :description="`Review verification ${verificationPublicId || ''}`"
    :ui="{ content: 'sm:max-w-7xl' }"
  >
    <template #body>
      <div class="space-y-4">
        <div v-if="isLoading" class="py-8 text-center">
          <p class="text-sm text-muted">Loading assets...</p>
        </div>

        <div v-else-if="assets.length === 0" class="py-8 text-center">
          <p class="text-sm text-warning">No manual-review assets found for this verification.</p>
        </div>

        <div v-else class="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
          <div
            v-for="asset in assets"
            :key="asset.id"
            class="space-y-2 rounded-lg border border-default p-3"
          >
            <div class="flex items-center justify-between gap-2">
              <p class="text-xs text-muted uppercase tracking-wide">{{ asset.type.replace('_', ' ') }}</p>
              <p class="text-xs text-muted">#{{ asset.id }}</p>
            </div>
            <p class="text-xs text-muted">{{ asset.subtype.replace('_', ' ') }}</p>
            <img
              :src="`/api/admin/manual-reviews/${verificationId}/assets/${asset.id}`"
              class="h-64 w-full rounded object-contain bg-muted"
              alt="Manual review asset"
            >
            <p class="text-xs text-muted">
              <NuxtTime
                :datetime="asset.created_at"
                locale="en"
                date-style="short"
                time-style="short"
                :hour12="false"
              />
            </p>
          </div>
        </div>

        <UFormField label="Review Notes">
          <UTextarea
            v-model="reviewNotes"
            class="w-full"
            placeholder="Add optional notes for this manual review"
            :rows="4"
          />
        </UFormField>

        <div class="flex gap-2 justify-end">
          <UButton
            variant="soft"
            :disabled="isSubmitting"
            @click="open = false"
          >
            Cancel
          </UButton>
          <UButton
            color="error"
            :loading="isSubmitting"
            @click="reject"
          >
            Reject
          </UButton>
          <UButton
            color="success"
            :loading="isSubmitting"
            @click="approve"
          >
            Approve
          </UButton>
        </div>
      </div>
    </template>
  </UModal>
</template>
