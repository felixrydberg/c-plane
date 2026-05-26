<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core'
import { NodeToolbar } from '@vue-flow/node-toolbar'

const props = defineProps<{
  id: string
  data: {
    label: string
    timeline: number
    date: string
    color: string
    isHead: boolean
    isDefault: boolean
    branchName: string
    branchId: string
    branchLabels: string[]
    onCreateBranch?: (id: string) => void
    onRepointBranch?: (id: string) => void
    onRemoveBranch?: (branchId: string) => void
  }
}>()
</script>

<template>
  <div class="relative size-5 group">
    <NodeToolbar :position="Position.Top" :offset="12">
      <div
        class="flex gap-0.5 bg-elevated border border-default rounded-lg shadow-lg p-0.5"
        @click.stop
      >
        <UButton
          size="xs"
          icon="i-heroicons:folder-plus"
          @click.stop="data.onCreateBranch?.(id)"
        >
          Branch
        </UButton>
        <UButton
          size="xs"
          icon="i-heroicons:arrow-uturn-left"
          @click.stop="data.onRepointBranch?.(id)"
        >
          Repoint
        </UButton>
        <UButton
          v-if="data.isHead && !data.isDefault"
          size="xs"
          icon="i-heroicons:trash"
          color="error"
          @click.stop="data.onRemoveBranch?.(data.branchId)"
        >
          Remove
        </UButton>
      </div>
    </NodeToolbar>
    <Handle type="target" :position="Position.Bottom" class="bg-transparent! border-0!" />

    <div class="absolute inset-0 flex items-center justify-center">
      <div
        v-if="data.isHead"
        class="absolute size-4 rounded-full opacity-15"
        :style="{ backgroundColor: data.color }"
      />
      <div
        class="rounded-full cursor-pointer transition-all duration-200 hover:scale-125"
        :class="[
          data.isHead ? 'size-3' : 'size-2 opacity-35 hover:opacity-60'
        ]"
        :style="{ backgroundColor: data.color }"
      />
    </div>

    <Handle type="source" :position="Position.Top" class="bg-transparent! border-0!" />

    <div
      v-if="data.branchLabels.length > 0"
      class="absolute left-full top-1/2 -translate-y-1/2 flex items-center gap-1.5 ml-2"
    >
      <span class="text-[11px] leading-none text-muted whitespace-nowrap capitalize">
        {{ data.branchLabels.join(', ') }}
      </span>
    </div>
  </div>
</template>
