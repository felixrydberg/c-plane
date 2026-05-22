<script setup lang="ts">
import type { NavigationMenuItem } from '@nuxt/ui'

const links = computed(() => {
  const items: NavigationMenuItem[] = [{
  type: "link",
  label: 'Overview',
  to: `/`,
  exact: true,
}, {
  type: "link",
  label: 'Manual Reviews',
  to: `/manual-reviews`,
  exact: true,
}, {
  type: "link",
  label: 'Customer Management',
  exact: true,
  open: true,
  children: [
    {
      type: "link",
      label: 'Users',
      to: `/users`,
      exact: true,
    }, {
      type: "link",
      label: 'Organizations',
      to: `/organizations`,
    }, {
      type: "link",
      label: 'API Keys',
      to: `/api-keys`,
    }
  ]
}, {
  type: "link",
  label: 'Infrastructure',
  exact: true,
  open: true,
  children: [
    {
      type: "link",
      label: 'Regions',
      to: `/regions`,
      exact: true,
    },
    {
      type: "link",
      label: 'Clusters',
      to: `/clusters`,
      exact: true,
    },
    {
      type: "link",
      label: 'S3 Providers',
      to: `/s3-providers`,
      exact: true,
    }
  ]
}]

  return items
})
</script>

<template>
  <UDashboardGroup unit="rem" class="min-h-screen">
    <UDashboardSidebar
      class="pb-1 pt-8"
      :ui="{root:'border-none'}"
    >
      <template #default>
        <UNavigationMenu
          :items="links"
          orientation="vertical"
          tooltip
          popover
        />
      </template>
    </UDashboardSidebar>
    <slot />
  </UDashboardGroup>
</template>