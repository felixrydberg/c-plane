<script setup lang="ts">
import type { NavigationMenuItem } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

const store = useStore();
const route = useRoute();
const open = ref(true);

const routeProjectId = computed(() => route.params.project_id as string | undefined)
const routeBranchId = computed(() => route.params.branch_id as string | undefined)
const navProjectId = computed(() => routeProjectId.value ?? store.project?.id)

const items = computed<NavigationMenuItem[]>(() => [
  {
    label: 'Overview',
    icon: ICONS.overview,
    to: `/${store.organization?.slug}`,
    exact: true,
  },
  {
    type: "label",
    label: 'Project Resources',
  },
  {
    label: 'Containers',
    icon: ICONS.containers,
    to: `/${store.organization?.slug}/containers${navProjectId.value ? `/${navProjectId.value}${routeBranchId.value ? `/${routeBranchId.value}` : ''}` : ''}`,
  },
  {
    label: 'Databases',
    icon: ICONS.databases,
    type: 'trigger',
    open: true,
    children: [
      {
        label: 'Stateful',
        to: `/${store.organization?.slug}/databases/stateful${navProjectId.value ? `/${navProjectId.value}` : ''}`,
      },
      {
        label: 'Serverless',
        disabled: true,
        to: `/${store.organization?.slug}/databases/serverless`,
      },
    ],
  },
  {
    label: 'Secrets',
    icon: ICONS.secrets,
    to: `/${store.organization?.slug}/secrets${navProjectId.value ? `/${navProjectId.value}` : ''}`,
  },
  {
    label: "Storage",
    icon: ICONS.storage,
    to: `/${store.organization?.slug}/storage${navProjectId.value ? `/${navProjectId.value}` : ''}`,
  },
  {
    type: "label",
    label: 'Organization',
  },
  {
    label: "Registry",
    icon: ICONS.registry,
    disabled: true,
    to: `/${store.organization?.slug}/registry`,
  },
  {
    label: 'Logs',
    icon: ICONS.logs,
    disabled: true,
    to: `/${store.organization?.slug}/logs`,
  },
  {
    label: 'Analytics',
    icon: ICONS.analytics,
    disabled: true,
    to: `/${store.organization?.slug}/analytics`,
  },
  {
    type: "label",
    label: 'Settings',
  },
      {
        label: 'General',
        icon: ICONS.general,
        to: `/${store.organization?.slug}/settings`,
        exact: true,
      },
      {
        label: 'Members',
        icon: ICONS.members,
        to: `/${store.organization?.slug}/settings/members`,
        exact: true,
      },
      {
        label: 'Authentication',
        icon: ICONS.authentication,
        to: `/${store.organization?.slug}/settings/authentication`,
        exact: true,
      },
] satisfies NavigationMenuItem[]);
</script>

<template>
  <div class="flex flex-1 min-h-screen">
    <USidebar
      v-model:open="open"
      collapsible="icon"
      rail
      :ui="{
        container: 'h-full',
        inner: 'bg-elevated/25 divide-transparent',
        body: 'py-0',
      }"
    >
      <template #header="{ collapsed }">
        <UiLogo :collapsed="collapsed" />
      </template>

      <template #default="slotProps">
        <dashboard-organizations-select :collapsed="!open" />
        <USeparator />
        <UNavigationMenu
          :key="slotProps?.state ?? 'expanded'"
          :collapsed="!open"
          :popover="!open"
          :items="items"
          orientation="vertical"
          :ui="{ link: 'p-1.5 overflow-hidden' }"
        />
      </template>

      <template #footer>
        <dashboard-user-overlay :collapsed="!open" />
      </template>
    </USidebar>

    <div class="flex-1 flex flex-col min-h-0">
      <div class="h-(--ui-header-height) shrink-0 flex items-center gap-2 px-4 border-b border-default">
        <UButton
          :icon="open ? ICONS.sidebarCloseLeft : ICONS.sidebarCloseRight"
          color="neutral"
          variant="ghost"
          aria-label="Toggle sidebar"
          @click="open = !open"
        />
        <dashboard-projects-nav />
      </div>

      <div class="flex-1 px-6 lg:px-8 py-4">
        <slot />
      </div>
    </div>
  </div>
</template>
