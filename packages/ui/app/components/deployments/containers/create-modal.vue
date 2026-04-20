<script setup lang="ts">
import { z } from 'zod';

const open = defineModel<boolean>('open');
const props = defineProps<{
  organizationId: string;
  refresh: () => void;
}>();

const toast = useToast();

type RegionOption = {
  id: string;
  slug: string;
  display_name: string;
  status: 'active' | 'inactive' | 'maintenance';
  routing_mode: 'active' | 'draining' | 'disabled';
};

const { data: regionsData, status: regionsStatus, refresh: refreshRegions } = await useFetch<RegionOption[]>(
  () => `/api/organization/${props.organizationId as ':organization_id'}/regions`
);

const regions = computed(() => regionsData.value ?? ([] as RegionOption[]));

watch(open, (isOpen) => {
  if (isOpen) {
    void refreshRegions();
  }
});

const createContainerSchema = z.object({
  name: z.string().min(1, 'Name is required'),
  image: z.string().min(1, 'Image is required'),
  port: z.number().int().min(1).max(65535),
  replicas: z.number().int().min(1),
  is_public: z.boolean(),
  region: z.string().min(1, 'Region is required'),
  health_check_path: z.string().min(1),
});

type CreateContainerState = z.infer<typeof createContainerSchema>;

const state = ref<CreateContainerState>({
  name: '',
  image: '',
  port: 80,
  replicas: 1,
  is_public: false,
  region: '',
  health_check_path: '/health',
});

const isSubmitting = ref(false);

async function onSubmit() {
  isSubmitting.value = true;
  try {
    // TODO: replace with $fetch call once API is ready
    await new Promise((resolve) => setTimeout(resolve, 500));
    toast.add({ title: 'Container deployment created', color: 'success' });
    open.value = false;
    props.refresh();
  } catch {
    toast.add({ title: 'Failed to create deployment', color: 'error' });
  } finally {
    isSubmitting.value = false;
  }
}

function onClose() {
  state.value = {
    name: '',
    image: '',
    port: 80,
    replicas: 1,
    is_public: false,
    region: '',
    health_check_path: '/health',
  };
}
</script>

<template>
  <UModal v-model:open="open" title="New Container Deployment" :ui="{ footer: 'justify-end' }" @close="onClose">
    <template #body>
      <UForm :state="state" :schema="createContainerSchema" class="flex flex-col gap-4" @submit.prevent="onSubmit">
        <UFormField label="Name" name="name" required>
          <UInput v-model="state.name" placeholder="e.g. api-gateway" class="w-full" />
        </UFormField>

        <UFormField label="Image" name="image" required>
          <UInput v-model="state.image" placeholder="e.g. nginx:latest" class="w-full" />
        </UFormField>

        <div class="grid grid-cols-2 gap-4">
          <UFormField label="Port" name="port" required>
            <UInput v-model.number="state.port" type="number" :min="1" :max="65535" class="w-full" />
          </UFormField>

          <UFormField label="Replicas" name="replicas" required>
            <UInput v-model.number="state.replicas" type="number" :min="1" class="w-full" />
          </UFormField>
        </div>

        <UFormField label="Region" name="region" required>
          <USelect
            v-model="state.region"
            :items="regions"
            value-attribute="id"
            label-attribute="display_name"
            :disabled="regionsStatus === 'pending' || regions.length === 0"
            class="w-full"
          />
        </UFormField>

        <UFormField label="Health Check Path" name="health_check_path" required>
          <UInput v-model="state.health_check_path" placeholder="/health" class="w-full" />
        </UFormField>

        <UFormField name="is_public">
          <UCheckbox v-model="state.is_public" label="Public endpoint" />
        </UFormField>
      </UForm>
    </template>

    <template #footer>
      <UButton variant="soft" color="neutral" label="Cancel" @click="open = false" />
      <UButton
        label="Create Deployment"
        :loading="isSubmitting"
        @click="onSubmit"
      />
    </template>
  </UModal>
</template>
