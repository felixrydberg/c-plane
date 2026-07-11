<script setup lang="ts">
const props = defineProps<{
  organizationId: string
  projectId: string
  branchId?: string | null
  eventTypePrefix: string
  targetId: string
}>()

interface ActivityItem {
  id: string
  action: string
  summary: string
  created_at: string
}

const url = computed(() => props.organizationId && props.projectId
  ? `/api/backend/organization/${props.organizationId}/events`
  : '')

const { data: activity, status, refresh } = await useFetch<ActivityItem[]>(url, {
  query: computed(() => ({
    project_id: props.projectId,
    branch_id: props.branchId || undefined,
    event_type_prefix: props.eventTypePrefix,
    target_id: props.targetId,
    limit: 6,
  })),
})

defineExpose({ refresh })
</script>

<template>
  <aside class="min-w-0 border-t border-default/60 px-5 py-6 xl:border-l xl:border-t-0">
    <h2 class="text-sm font-semibold">Recent Activity</h2>

    <div v-if="status === 'pending'" class="flex items-center gap-2 py-8 text-sm text-muted">
      <UIcon name="i-lucide-loader-circle" class="size-4 animate-spin" />
      Loading activity&hellip;
    </div>

    <div v-else-if="!activity?.length" class="py-8 text-sm text-muted">
      No recent activity for this branch.
    </div>

    <ol v-else class="mt-6 space-y-6">
      <li v-for="(item, index) in activity" :key="item.id" class="relative pl-7">
        <span v-if="index < activity.length - 1" class="absolute left-2 top-5 h-[calc(100%+1.5rem)] w-px bg-default/50" />
        <UIcon name="i-heroicons:check-circle" class="absolute left-0 top-0.5 size-4 text-success" />
        <p class="text-sm font-medium leading-5">{{ item.summary }}</p>
        <p class="mt-1 font-mono text-[11px] capitalize text-muted">{{ item.action }}</p>
        <NuxtTime :datetime="item.created_at" relative class="mt-1 block text-xs text-muted" />
      </li>
    </ol>
  </aside>
</template>
