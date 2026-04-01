<script setup lang="ts" generic="T extends { label: string; value: string }[] | { label: string; value: string } = { label: string; value: string }[]">
import { refDebounced } from '@vueuse/core';

const props = withDefaults(defineProps<{
  single?: boolean;
  class?: string;
  placeholder?: string;
  excludeIds?: string[];
  excludeRequester?: boolean;
  excludeExternal?: boolean;
}>(), {
  single: false,
  excludeRequester: true,
  excludeExternal: true,
  class: "w-full",
  placeholder: "Search members...",
  excludeIds: () => [],
});

const selectedUser = defineModel<T>('user', {
  required: false,
});

const store = useStore();
const searchTerm = ref('');
const searchTermDebounced = refDebounced(searchTerm, 300);

const {
  data,
  status,
  error
} = await useFetch(`/api/organization/${store.organization?.id as ':organization_id'}/members`, {
  method: 'GET',
  query: {
    limit: 10,
    search: searchTermDebounced,
    excludeRequester: props.excludeRequester,
    excludeExternal: props.excludeExternal,
    excludeIds: props.excludeIds.join(','),
  }
});

if (error.value) {
  throw createError({
    statusCode: error.value?.status || 500,
    statusMessage: 'Failed to load members',
  });
}

const filteredItems = computed(() => {
  return (data.value?.data || [])
    .map(member => ({ 
      label: member.user.name + ` (${member.user.email})`, 
      value: member.id 
    }));
});
</script>

<template>
  <USelectMenu
    v-model="selectedUser"
    v-model:search-term="searchTerm"
    :loading="status === 'pending'"
    :multiple="!props.single"
    ignore-filter
    :class="props.class"
    :placeholder="props.placeholder"
    :items="filteredItems"
  >
    <template #empty>
      <div class="p-4 h-16 text-center flex justify-center items-center text-sm text-default/50">
        No members found
      </div>
    </template>
  </USelectMenu>
</template>
