<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core'

defineProps<{
  id: string
  data: {
    label: string
    branchName: string
    timeline: number
    date: string
    color: string
    isHead: boolean
    branches: { id: string; name: string; isDefault: boolean; exists: boolean }[]
    branchLabels: string[]
    isSelected?: boolean
  }
}>()
</script>

<template>
  <div
    class="relative cursor-pointer"
    :class="{ 'z-10': data.isSelected }"
  >
    <div
      class="relative border border-dashed rounded-full flex flex-row items-center whitespace-nowrap transition-all hover:shadow-md overflow-visible"
      :class="[
        data.branchLabels.length > 0 ? '' : 'border-default',
        data.isSelected ? 'ring-2 ring-primary ring-offset-2 ring-offset-default z-10' : '',
      ]"
      :style="data.branchLabels.length > 0 ? { borderColor: data.color } : {}"
    >
      <Handle type="target" :position="Position.Left" class="bg-transparent! border-0!" />

      <!-- Dot + revision label -->
      <span class="pl-2.5 pr-2 py-1 text-xs text-muted flex items-center gap-1.5">
        <div
          class="rounded-full flex-shrink-0"
          :class="[data.branchLabels.length > 0 ? 'size-2.5' : 'size-2', data.branchLabels.length === 0 ? 'bg-muted' : '']"
          :style="data.branchLabels.length > 0 ? { backgroundColor: data.color } : {}"
        />
        {{ data.label }}
      </span>

      <!-- Branch tag (optional) -->
      <span
        v-if="data.branchLabels.length > 0"
        class="border-l border-dashed pl-2 pr-2.5 py-1 text-xs font-medium"
        :style="{ borderColor: data.color, color: data.color, backgroundColor: data.color + '15' }"
      >
        {{ data.branchLabels[0] }}
        <span v-if="data.branchLabels.length > 1" class="opacity-50 ml-0.5">+{{ data.branchLabels.length - 1 }}</span>
      </span>

      <Handle type="source" :position="Position.Right" class="bg-transparent! border-0!" />
    </div>
  </div>
</template>
