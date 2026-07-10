<script setup lang="ts">
import { z } from 'zod';
import { useClipboard } from '@vueuse/core';

const open = defineModel<boolean>("open");
const props = defineProps<{
  apiKeyId?: string;
  organizationId: string;
  refresh: () => void;
  apiKey?: string;
}>();

const toast = useToast();

const apiKeyStateSchema = z.object({
  name: z.string(),
  allowed_ips: z.string().optional(),
  scopes: z.record(z.string(), z.boolean()),
});

type ApiKeyState = z.infer<typeof apiKeyStateSchema>;

const { data, status, refresh } = await useFetch(`/api/organization/${props.organizationId as ':organization_id'}/api-keys/${props.apiKeyId as ':api_key_id'}`);
type ApiKeyDetails = NonNullable<typeof data.value>;

const apiKeyDetails = computed(() => data.value as ApiKeyDetails | undefined);
const isLoading = computed(() => status.value === 'pending');

const state = ref<ApiKeyState>({
  name: '',
  allowed_ips: '',
  scopes: {},
});

const { copy } = useClipboard();

const hasChanged = computed(() => {
  if (!apiKeyDetails.value) return false;
  return (
    state.value.name !== apiKeyDetails.value.name ||
    (state.value.allowed_ips ?? '') !== (apiKeyDetails.value.allowed_ips ?? '') ||
    JSON.stringify(state.value.scopes) !== JSON.stringify(
      Object.fromEntries(
        apiKeyDetails.value.scopes.map(scope => [scope, true])
      )
    )
  );
});

watch(data, (newData) => {
  if (newData) {
    const keyDetails = newData as ApiKeyDetails;
    state.value = {
      name: keyDetails.name,
      allowed_ips: keyDetails.allowed_ips ?? '',
      scopes: Object.fromEntries(
        keyDetails.scopes.map(scope => [scope, true])
      ),
    };
  }
}, { immediate: true });

const onSubmit = async () => {
  if (!apiKeyDetails.value) return;

  try {
    await $fetch(`/api/organization/${props.organizationId as ':organization_id'}/api-keys/${props.apiKeyId as ':api_key_id'}`, {
      method: 'PUT',
      body: {
        name: state.value.name,
        scopes: state.value.scopes,
        allowed_ips: state.value.allowed_ips || null,
      },
    });
    toast.add({
      title: 'Success',
      description: 'API key updated',
      color: 'success'
    });
    props.refresh();
    refresh();
    open.value = false;
  } catch (error) {
    toast.add({
      title: 'An Error accured',
      description: error instanceof Error ? error.message : 'Failed to update API key',
      color: 'error'
    });
  }
};
const copyKey = async () => {
  const keyToCopy = props.apiKey;
  if (!keyToCopy) return;
  await copy(keyToCopy);
  toast.add({
    title: 'Success',
    description: 'API key copied to clipboard',
    color: 'success'
  });
};

const getExpiredDate = (created_at: string, expires_in: number) => {
  const createdDate = new Date(created_at);
  createdDate.setSeconds(createdDate.getSeconds() + expires_in);
  return createdDate.toLocaleString();
};
</script>

<template>
  <UModal
    v-model:open="open"
    title="API Key Details"
    description="View API key information and scopes"
  >
    <template #body>
      <div v-if="isLoading" class="py-8 text-center">
        <UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" />
      </div>

      <UForm
        v-else-if="apiKeyDetails"
        class="space-y-4"
        :state="state"
        :schema="apiKeyStateSchema"
        @submit.prevent="onSubmit"
      >
        <UFormField label="Name">
          <UInput
            v-model="state.name"
            class="w-full"
          />
        </UFormField>

        <UFormField v-if="props.apiKey" label="API Key">
          <div class="flex items-center gap-2 w-full">
            <UInput :model-value="props.apiKey" readonly class="w-full font-mono text-xs" />
            <UButton variant="soft" leading-icon="i-heroicons:clipboard" @click="copyKey">
              Copy
            </UButton>
          </div>
        </UFormField>

        <UFormField label="Allowed IPs" description="Comma-separated list of allowed IPs. Leave empty to allow all.">
          <UInput
            v-model="state.allowed_ips"
            placeholder="e.g. 192.168.1.1, 10.0.0.1"
            class="w-full"
          />
        </UFormField>

        <div class="space-y-2">
          <div class="flex justify-between">
            <p class="text-sm text-muted">Created At:</p>
            <NuxtTime
              class="text-sm text-muted"
              :datetime="apiKeyDetails.created_at"
              locale="en"
              date-style="short"
              time-style="short"
            />
          </div>
          <div class="flex justify-between">
            <p class="text-sm text-muted">Expires At:</p>
            <NuxtTime
              v-if="apiKeyDetails.expires_at && apiKeyDetails.expires_at > 0"
              class="text-sm text-muted"
              :datetime="getExpiredDate(apiKeyDetails.created_at, apiKeyDetails.expires_at)"
              locale="en"
              date-style="short"
              time-style="short"
            />
            <p v-else class="text-sm text-muted">Never</p>
          </div>
        </div>

        <OrganizationApiKeyScopes
          v-if="data"
          v-model="state.scopes"
        />

        <div class="flex justify-end gap-3 pt-2">
          <UButton
            variant="ghost"
            color="neutral"
            type="button"
            @click="open = false"
          >
            Cancel
          </UButton>
          <UButton
            :disabled="!hasChanged"
            type="submit"
          >
            Update
          </UButton>
        </div>
      </UForm>
    </template>
  </UModal>
</template>
