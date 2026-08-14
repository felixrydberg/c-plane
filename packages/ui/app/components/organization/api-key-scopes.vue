<script setup lang="ts">
  import { API_KEY_SCOPE_VALUES } from '@cplane/migrations/utils';

  const selectedScopes = defineModel<Record<string, boolean>>({ default: () => ({}) });
  const emits = defineEmits(['update']);
</script>

<template>
  <div class="space-y-1">
    <p class="text-sm text-muted">Select scopes for this API key:</p>
    <div class="space-y-1">
      <div v-for="scope in API_KEY_SCOPE_VALUES" :key="scope" class="text-sm">
        <UCheckbox
          v-model="selectedScopes[scope]"
          :label="scope"
          @change="emits('update', selectedScopes)"
        />
      </div>
    </div>
  </div>
</template>
