<script setup lang="ts">
import type { NavigationMenuItem } from '@nuxt/ui'

const store = useStore();
const open = ref(false);

const links = computed(() => [{
  type: "link",
  label: 'Overview',
  icon: 'i-heroicons:rectangle-group',
  to: `/${store.organization?.slug}/`,
  onSelect: () => {
    open.value = false
  },
  exact: true,
}, {
  type: "link",
  label: 'Deployments',
  icon: "i-heroicons:rectangle-stack",
  onSelect: () => {
    open.value = false
  },
  open: true,
  children: [
    {
      type: "link",
      label: 'Containers',
      to: `/${store.organization?.slug}/deployments/containers`,
      onSelect: () => {
        open.value = false
      }
    },
  ]
}, {
  type: "link",
  label: 'Settings',
  icon: "i-heroicons:adjustments-horizontal",
  onSelect: () => {
    open.value = false
  },
  open: true,
  children: [
    {
      type: "link",
      label: 'General',
      to: `/${store.organization?.slug}/settings`,
      exact: true,
      onSelect: () => {
        open.value = false
      }
    },
    {
      type: "link",
      label: 'Members',
      to: `/${store.organization?.slug}/settings/members`,
      exact: true,
      onSelect: () => {
        open.value = false
      }
    },
    {
      type: "link",
      label: 'Authentication',
      to: `/${store.organization?.slug}/settings/authentication`,
      exact: true,
      onSelect: () => {
        open.value = false
      }
    }
  ]
}] as NavigationMenuItem[]);

const groups = computed(() => {
  const items = [
    ...links.value,
  ];

  return [{
    id: 'general' as const,
    label: 'General',
    items,
  }, {
    id: 'links' as const,
    label: 'Go to',
    items,
  }];
});
</script>

<template>
  <UDashboardGroup unit="rem" class="min-h-screen">
    <UDashboardSidebar
      class="py-3"
      :ui="{
        root: 'border-none',
        body: 'py-0',
        footer: 'py-0'
      }"
    >
      <template #default>
        <div class="flex justify-start">
          <UiLogo size="lg" />
        </div>
        <dashboard-organizations-select :collapsed="false" />
        <USeparator />
        <UDashboardSearchButton variant="soft" />
        <UNavigationMenu :items="links" orientation="vertical" tooltip popover />
      </template>
        <template #footer>
        <dashboard-user-overlay />
      </template>
    </UDashboardSidebar>
    <UDashboardSearch :groups="groups" />
    <slot />
  </UDashboardGroup>
</template>
