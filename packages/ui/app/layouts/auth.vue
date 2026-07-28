<script setup lang="ts">
import type { NavigationMenuItem } from '@nuxt/ui'
import DashboardProjectsDeploymentAlert from '~/components/dashboard/projects/deployment-alert.vue'
import DashboardRegistryMaintenanceAlert from '~/components/dashboard/registry-maintenance-alert.vue'
import { ICONS } from '~/utils/icons'

const store = useStore();
const route = useRoute();
const open = ref(true);

const routeProjectId = computed(() => route.params.project_id as string | undefined)
const routeEnvironmentId = computed(() => route.params.environment_id as string | undefined)
const navProjectId = computed(() => routeProjectId.value ?? store.project?.id)
const draftRevisionQuery = computed(() =>
  route.query.revision === store.environment?.draft_timeline
    ? `?revision=${store.environment?.draft_timeline}`
    : ''
)

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
    to: `/${store.organization?.slug}/containers${navProjectId.value ? `/${navProjectId.value}${routeEnvironmentId.value ? `/${routeEnvironmentId.value}` : ''}` : ''}${draftRevisionQuery.value}`,
  },
  {
    label: 'Databases',
    icon: ICONS.databases,
    open: true,
    children: [
      {
        label: 'Postgres',
        to: `/${store.organization?.slug}/databases/postgres${navProjectId.value ? `/${navProjectId.value}` : ''}`,
      },
    ]
  },
  {
    label: 'Secrets',
    icon: ICONS.secrets,
    disabled: true,
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
      {
        label: 'Audit Log',
        icon: ICONS.logs,
        to: `/${store.organization?.slug}/settings/audit-log`,
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
        inner: 'bg-default divide-default',
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
          :ui="{
            label: 'font-mono text-[10px] font-normal uppercase tracking-[0.08em] text-[#797979]',
            link: 'p-1.5 overflow-hidden',
          }"
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
        <dashboard-projects-nav />
      </div>

      <DashboardRegistryMaintenanceAlert
        v-if="store.organization"
        :organization-id="store.organization.id"
      />
      <DashboardProjectsDeploymentAlert v-if="store.environment" />

      <div class="flex-1 bg-default px-6 py-6 lg:px-8">
        <slot />
      </div>
    </div>
  </div>
</template>
