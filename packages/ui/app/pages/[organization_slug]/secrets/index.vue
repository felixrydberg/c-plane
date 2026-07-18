<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

const store = useStore()
const route = useRoute()
const slug = computed(() => route.params.organization_slug?.toString())

const projectDropdownItems = computed<DropdownMenuItem[][]>(() => {
  const list: DropdownMenuItem[] = store.projects.map(p => ({
    label: p.name,
    icon: ICONS.folder,
    onSelect() {
      navigateTo(`/${slug.value}/secrets/${p.id}`)
    },
  }))
  return [list]
})
</script>

<template>
  <div class="flex w-full max-w-[1500px] flex-col gap-5 mx-auto">
    <div class="flex flex-col gap-4 border-b border-default/60 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <h1 class="text-2xl font-semibold">Secrets</h1>
        <p class="text-muted text-sm mt-1">Manage environment variables and secrets across projects.</p>
      </div>
    </div>

    <div class="flex flex-col items-center justify-center py-16 gap-3 text-center border border-dashed border-default rounded-lg">
      <UIcon :name="ICONS.secrets" class="size-10 text-muted" />
      <p class="text-muted">Select a project to manage its secrets.</p>
      <UDropdownMenu :items="projectDropdownItems" :content="{ align: 'center' }">
        <UButton label="Choose Project" variant="soft" color="neutral" />
      </UDropdownMenu>
    </div>
  </div>
</template>
