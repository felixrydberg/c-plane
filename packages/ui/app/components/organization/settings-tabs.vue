<script setup lang="ts">
const store = useStore()
const route = useRoute()
const base = computed(() => `/${store.organization?.slug}/settings`)

const links = computed(() => [
  { label: 'General', to: base.value },
  { label: 'Members', to: `${base.value}/members` },
  { label: 'Authentication', to: `${base.value}/authentication` },
  { label: 'Audit log', to: `${base.value}/audit-log` },
])
</script>

<template>
  <nav aria-label="Organization settings" class="overflow-x-auto border-b border-default">
    <div class="flex min-w-max gap-7">
      <NuxtLink
        v-for="link in links"
        :key="link.to"
        :to="link.to"
        :class="[
          'border-b-2 px-0.5 pb-3 text-sm transition-colors',
          route.path === link.to ? 'border-primary text-primary' : 'border-transparent text-muted hover:text-default',
        ]"
      >
        {{ link.label }}
      </NuxtLink>
    </div>
  </nav>
</template>
