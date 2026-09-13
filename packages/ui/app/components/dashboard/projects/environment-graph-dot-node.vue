<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core'

defineProps<{
  id: string
  data: {
    label: string
    environmentName: string
    timeline: number
    date: string
    color: string
    isHead: boolean
    environments: { id: string; name: string; isDefault: boolean; exists: boolean }[]
    environmentLabels: string[]
    forkLabel?: string
    continuesNewer?: boolean
    isSelected?: boolean
  }
}>()

function activate(event: KeyboardEvent) {
  if (event.key !== 'Enter' && event.key !== ' ') return
  event.preventDefault()
  ;(event.currentTarget as HTMLElement).click()
}
</script>

<template>
  <div
    class="relative cursor-pointer"
    :class="{ 'z-10': data.isSelected }"
    role="button"
    tabindex="0"
    :aria-label="id.startsWith('boundary-') ? `Show ${data.label.toLowerCase()}` : `${data.label}, ${data.environmentName || 'No environment'}, ${data.date}`"
    :aria-pressed="id.startsWith('boundary-') ? undefined : data.isSelected ? 'true' : 'false'"
    :title="id.startsWith('boundary-') ? undefined : data.label"
    @keydown="activate"
  >
    <span
      v-if="data.forkLabel"
      class="pointer-events-none absolute bottom-full left-2 mb-1 max-w-40 truncate text-[10px] font-medium text-muted"
      :title="data.forkLabel"
    >
      {{ data.forkLabel }}
    </span>
    <span
      v-if="data.continuesNewer && !id.startsWith('boundary-')"
      class="pointer-events-none absolute bottom-full right-2 mb-1 text-[10px] font-medium text-muted"
    >
      Newer revisions
    </span>
    <div
      class="relative border border-dashed rounded-full flex flex-row items-center whitespace-nowrap bg-default transition-all hover:shadow-md overflow-hidden"
      :class="[
        data.isSelected ? 'ring-2 ring-neutral ring-offset-2 ring-offset-default z-10' : '',
      ]"
      :style="{ borderColor: data.color }"
    >
      <Handle type="target" :position="Position.Left" class="bg-transparent! border-0!" />

      <!-- Dot + revision label -->
      <span class="pl-2.5 pr-2 py-1 text-sm font-medium text-default flex items-center gap-1.5">
        <div
          class="rounded-full flex-shrink-0"
          :class="data.environmentLabels.length > 0 ? 'size-2.5' : 'size-2'"
          :style="{ backgroundColor: data.color }"
        />
        <template v-if="id.startsWith('boundary-')">{{ data.label }}</template>
        <template v-else>Revision {{ data.timeline }}</template>
      </span>

      <!-- Environment tag (optional) -->
      <span
        v-if="data.environmentLabels.length > 0"
        class="border-l border-dashed pl-2 pr-2.5 py-1 text-xs font-medium"
        :style="{ borderColor: data.color, color: data.color, backgroundColor: data.color + '15' }"
      >
        {{ data.environmentLabels[0] }}
        <span v-if="data.environmentLabels.length > 1" class="opacity-50 ml-0.5">+{{ data.environmentLabels.length - 1 }}</span>
      </span>

      <Handle type="source" :position="Position.Right" class="bg-transparent! border-0!" />
    </div>
  </div>
</template>
