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
const draftRevisionQuery = computed(() => {
  const draftTimeline = store.environment?.draft_timeline
  return draftTimeline !== undefined && route.query.revision === draftTimeline
    ? `?revision=${draftTimeline}`
    : ''
})

const items = computed<NavigationMenuItem[]>(() => {
  const all: NavigationMenuItem[] = [
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
  ];

  // Hide nav entries the active member has no scope for; the backend stays the boundary.
  const requiredScope: Record<string, string> = {
    Containers: 'container:read',
    Databases: 'database:postgres:read',
    Storage: 'bucket:read',
    Registry: 'registry:read',
    General: 'org:update',
    Authentication: 'api-key:manage',
    'Audit Log': 'event:read',
  };

  return all.filter((item) => {
    const label = item.label as string;
    if (label === 'Members') {
      return store.isOwner || store.can('member:invite') || store.can('member:remove');
    }
    const scope = label ? requiredScope[label] : undefined;
    if (!scope) return true;
    return store.can(scope);
  });
});
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
