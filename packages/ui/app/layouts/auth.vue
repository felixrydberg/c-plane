<script setup lang="ts">
import type { NavigationMenuItem } from '@nuxt/ui'
import DashboardProjectsDeploymentAlert from '~/components/dashboard/projects/deployment-alert.vue'
import { ICONS } from '~/utils/icons'

const store = useStore();
const route = useRoute();
const open = ref(true);
const isOwner = computed(() => store.organization?.member?.role === 'owner')

const routeProjectId = computed(() => route.params.project_id as string | undefined)
const routeEnvironmentId = computed(() => route.params.environment_id as string | undefined)
const navProjectId = computed(() => routeProjectId.value ?? store.project?.id)
const revisionQuery = computed(() => {
  const revision = typeof route.query.revision === 'string' ? route.query.revision : undefined
  if (revision !== undefined && (revision === store.environment?.draft_timeline || revision === store.environment?.deployed_timeline)) {
    return `?revision=${revision}`
  }
  if (store.environment && store.environment.draft_timeline !== store.environment.deployed_timeline) {
    return `?revision=${store.environment.draft_timeline}`
  }
  return ''
})

const mainItems = computed<NavigationMenuItem[]>(() => [
  {
    label: 'Overview',
    icon: ICONS.overview,
    to: `/${store.organization?.slug}`,
    exact: true,
  },
  {
    type: 'label',
    label: 'Observe',
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
    type: 'label',
    label: 'Build',
  },
  {
    label: 'Compute',
    icon: ICONS.bolt,
    trailingIcon: ICONS.chevronRight,
    open: true,
    children: [
      {
        label: 'Containers',
        to: `/${store.organization?.slug}/compute/containers${navProjectId.value ? `/${navProjectId.value}${routeEnvironmentId.value ? `/${routeEnvironmentId.value}` : ''}` : ''}${revisionQuery.value}`,
      },
    ],
  },
  {
    label: 'Storage & Databases',
    icon: ICONS.databases,
    trailingIcon: ICONS.chevronRight,
    open: true,
    children: [
      {
        label: 'Object Storage',
        to: `/${store.organization?.slug}/storage${navProjectId.value ? `/${navProjectId.value}` : ''}`,
      },
      {
        label: 'Registry',
        to: `/${store.organization?.slug}/registry${navProjectId.value ? `/${navProjectId.value}` : ''}`,
      },
      {
        label: 'Postgres',
        to: `/${store.organization?.slug}/databases/postgres${navProjectId.value ? `/${navProjectId.value}` : ''}`,
      },
    ]
  },
  {
    label: 'Integrations',
    icon: ICONS.link,
    trailingIcon: ICONS.chevronRight,
    open: true,
    children: [
      {
        label: 'GitHub',
        to: `/${store.organization?.slug}/integrations/github`,
        exact: true,
      },
      {
        label: 'External Registries',
        to: `/${store.organization?.slug}/integrations/external-registries`,
        exact: true,
      },
    ],
  },
] satisfies NavigationMenuItem[]);

const accountItems = computed<NavigationMenuItem[]>(() => [{
  label: 'Manage organization',
  trailingIcon: ICONS.chevronRight,
  children: [
    ...(isOwner.value ? [{
      label: 'General',
      to: `/${store.organization?.slug}/settings`,
      exact: true,
    }] : []),
    {
      label: 'Members',
      to: `/${store.organization?.slug}/settings/members`,
      exact: true,
    },
    ...(isOwner.value ? [{
      label: 'API Keys',
      to: `/${store.organization?.slug}/settings/authentication`,
      exact: true,
    }] : []),
    {
      label: 'Audit Log',
      to: `/${store.organization?.slug}/settings/audit-log`,
      exact: true,
    },
    {
      label: 'Registry',
      to: `/${store.organization?.slug}/settings/registry`,
      exact: true,
    },
  ],
}]);

const navigationUi = {
  list: 'space-y-1',
  item: 'px-0',
  label: 'px-3 pb-2 pt-5 text-sm font-normal text-muted first:pt-4',
  link: 'h-[34px] gap-3 overflow-hidden rounded-md px-3 py-0 font-medium text-default text-sm hover:before:bg-elevated',
  childItem: '[&>a]:font-medium',
  childLink: 'px-3 py-2 text-sm',
  linkLeadingIcon: 'size-5 text-dimmed',
  linkTrailingIcon: 'size-4 rotate-0 text-muted opacity-60 transition-transform duration-200 ease-out group-hover:text-default group-hover:opacity-100 group-data-[state=open]:rotate-90',
}
</script>

<template>
  <div class="flex flex-1 min-h-screen">
    <USidebar
      v-model:open="open"
      collapsible="icon"
      rail
      :ui="{
        container: 'h-full',
        inner: 'bg-default divide-y-0',
        body: 'px-2 py-0',
      }"
    >
      <template #header="{ collapsed }">
        <UiLogo :collapsed="collapsed" />
      </template>

      <template #default="slotProps">
        <dashboard-organizations-select :collapsed="!open" />
        <dashboard-projects-nav :collapsed="!open" />
        <USeparator class="px-2" />
        <UNavigationMenu
          :key="`main-${slotProps?.state ?? 'expanded'}`"
          :collapsed="!open"
          :popover="!open"
          :items="mainItems"
          orientation="vertical"
          :ui="navigationUi"
        />

        <USeparator class="px-2" />
        <UNavigationMenu
          :key="`account-${open ? 'expanded' : 'collapsed'}`"
          :collapsed="!open"
          :popover="!open"
          :items="accountItems"
          orientation="vertical"
          :ui="navigationUi"
        />
      </template>

      <template #footer>
        <dashboard-user-overlay :collapsed="!open" />
      </template>
    </USidebar>

    <div class="flex-1 flex flex-col min-h-0">
      <div class="h-(--ui-header-height) shrink-0 flex items-center gap-2 border-b border-default bg-default px-4">
        <UButton
          :icon="open ? ICONS.sidebarCloseLeft : ICONS.sidebarCloseRight"
          color="neutral"
          variant="ghost"
          aria-label="Toggle sidebar"
          @click="open = !open"
        />
        <nav
          v-if="store.breadcrumbs.length"
          aria-label="Breadcrumb"
          class="min-w-0 flex flex-1 items-center gap-1 overflow-hidden text-sm"
        >
          <template v-for="(breadcrumb, index) in store.breadcrumbs" :key="`${breadcrumb.label}-${index}`">
            <UIcon
              v-if="index > 0"
              :name="ICONS.chevronRight"
              class="size-4 shrink-0 text-dimmed"
              aria-hidden="true"
            />
            <NuxtLink
              v-if="breadcrumb.to && index < store.breadcrumbs.length - 1"
              :to="breadcrumb.to"
              class="max-w-48 truncate text-muted transition-colors hover:text-default"
            >
              {{ breadcrumb.label }}
            </NuxtLink>
            <span v-else class="max-w-56 truncate font-medium">
              {{ breadcrumb.label }}
            </span>
          </template>
        </nav>
      </div>

      <DashboardProjectsDeploymentAlert />

      <div class="flex-1 bg-default px-6 py-6 lg:px-8">
        <slot />
      </div>
    </div>
  </div>
</template>
