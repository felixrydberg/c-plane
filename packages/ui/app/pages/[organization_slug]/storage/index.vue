<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const projectId = computed(() => store.project?.id || store.projects[0]?.id)

if (projectId.value) {
  await navigateTo(`/${route.params.organization_slug}/storage/${projectId.value}`)
}
</script>

<template>
  <div class="flex flex-col items-center justify-center py-16 gap-3 text-center">
    <UIcon :name="ICONS.storage" class="size-10 text-muted" />
    <p class="text-muted">Select a project to manage its storage.</p>
  </div>
</template>
