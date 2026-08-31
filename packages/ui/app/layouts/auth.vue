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
const draftRevisionQuery = computed(() => {
  const draftTimeline = store.environment?.draft_timeline
  return draftTimeline !== undefined && route.query.revision === draftTimeline
    ? `?revision=${draftTimeline}`
    : ''
})

type RecentSection = Pick<NavigationMenuItem, 'label' | 'to'> & {
  description?: string
}

const recentSectionDescriptions: Record<string, string> = {
  'C1 - Containers': 'Compute',
  'S1 - Object Storage': 'Storage & Databases',
  'S2 - Registry': 'Storage & Databases',
  'External Registries': 'Integrations',
  'D1 - Postgres': 'Storage & Databases',
  General: 'Manage account',
  Members: 'Manage account',
  'API Keys': 'Manage account',
  'Audit Log': 'Manage account',
}
const recentSectionLabelAliases: Record<string, string> = {
  Containers: 'C1 - Containers',
  'O2 Object Storage': 'S1 - Object Storage',
  'O2 Registry': 'S2 - Registry',
  Postgres: 'D1 - Postgres',
}

const recentSections = ref<RecentSection[]>([])
const recentSectionsReady = ref(false)
const skippedRecentPath = ref<string | null>(null)
const recentSectionsStorageKey = computed(() => {
  const organizationSlug = store.organization?.slug
  if (!organizationSlug) return ''
  return `cplane:recent-sections:${store.user?.id ?? 'anonymous'}:${organizationSlug}`
})

function currentRecentSection(): RecentSection | null {
  const organizationSlug = store.organization?.slug
  if (!organizationSlug) return null

  const path = route.path.slice(`/${organizationSlug}`.length)
  if (path.startsWith('/containers')) return { label: 'C1 - Containers', description: 'Compute', to: route.fullPath }
  if (path.startsWith('/storage')) return { label: 'S1 - Object Storage', description: 'Storage & Databases', to: route.fullPath }
  if (path.startsWith('/integrations/github')) return { label: 'GitHub', description: 'Integrations', to: route.fullPath }
  if (path.startsWith('/integrations/external-registries')) return { label: 'External Registries', description: 'Integrations', to: route.fullPath }
  if (path.startsWith('/registry')) return { label: 'S2 - Registry', description: 'Storage & Databases', to: route.fullPath }
  if (path.startsWith('/databases/postgres')) return { label: 'D1 - Postgres', description: 'Storage & Databases', to: route.fullPath }
  if (path.startsWith('/settings/members')) return { label: 'Members', description: 'Manage account', to: route.fullPath }
  if (path.startsWith('/settings/authentication')) return { label: 'API Keys', description: 'Manage account', to: route.fullPath }
  if (path.startsWith('/settings/audit-log')) return { label: 'Audit Log', description: 'Manage account', to: route.fullPath }
  if (path.startsWith('/settings')) return { label: 'General', description: 'Manage account', to: route.fullPath }
  return null
}

function loadRecentSections() {
  const key = recentSectionsStorageKey.value
  if (!key) {
    recentSections.value = []
    return
  }

  try {
    const stored = JSON.parse(localStorage.getItem(key) ?? '[]') as RecentSection[]
    recentSections.value = Array.isArray(stored)
      ? stored
        .filter(section => typeof section?.label === 'string' && typeof section?.to === 'string')
        .map(section => ({ ...section, label: recentSectionLabelAliases[section.label] ?? section.label }))
        .slice(0, 5)
      : []
  } catch {
    recentSections.value = []
  }
}

function trackRecentSection() {
  const section = currentRecentSection()
  const key = recentSectionsStorageKey.value
  if (!section || !key) return

  recentSections.value = [
    section,
    ...recentSections.value.filter(item => item.label !== section.label),
  ].slice(0, 5)
  localStorage.setItem(key, JSON.stringify(recentSections.value))
}

const recentItems = computed<NavigationMenuItem[]>(() => recentSections.value.length
  ? recentSections.value.map(section => ({
      ...section,
      description: section.description ?? recentSectionDescriptions[section.label ?? ''],
      slot: 'recent-child',
      class: 'min-h-12',
      onSelect: () => { skippedRecentPath.value = section.to ?? null },
    }))
  : [{ label: 'No recent sections', disabled: true }])

onMounted(() => {
  loadRecentSections()
  recentSectionsReady.value = true
  trackRecentSection()
})

watch(() => route.fullPath, () => {
  if (!recentSectionsReady.value) return
  if (skippedRecentPath.value === route.fullPath) {
    skippedRecentPath.value = null
    return
  }
  skippedRecentPath.value = null
  trackRecentSection()
})

watch(recentSectionsStorageKey, () => {
  if (!recentSectionsReady.value) return
  loadRecentSections()
  trackRecentSection()
})

const mainItems = computed<NavigationMenuItem[]>(() => [
  {
    label: 'Overview',
    icon: ICONS.overview,
    to: `/${store.organization?.slug}`,
    exact: true,
  },
  {
    label: 'Recent',
    icon: ICONS.revision,
    trailingIcon: ICONS.chevronRight,
    open: true,
    children: recentItems.value,
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
        label: 'C1 - Containers',
        to: `/${store.organization?.slug}/containers${navProjectId.value ? `/${navProjectId.value}${routeEnvironmentId.value ? `/${routeEnvironmentId.value}` : ''}` : ''}${draftRevisionQuery.value}`,
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
        label: 'S1 - Object Storage',
        to: `/${store.organization?.slug}/storage${navProjectId.value ? `/${navProjectId.value}` : ''}`,
      },
      {
        label: 'S2 - Registry',
        to: `/${store.organization?.slug}/registry`,
      },
      {
        label: 'D1 - Postgres',
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
  open: false,
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
        <USeparator />
        <UNavigationMenu
          :key="`main-${slotProps?.state ?? 'expanded'}`"
          :collapsed="!open"
          :popover="!open"
          :items="mainItems"
          orientation="vertical"
          :ui="navigationUi"
        >
          <template #recent-child="{ item }">
            <div class="min-w-0">
              <span class="block truncate text-sm">{{ item.label }}</span>
              <span v-if="item.description" class="block truncate text-xs text-muted">{{ item.description }}</span>
            </div>
          </template>
        </UNavigationMenu>

        <USeparator class="my-2" />
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

      <DashboardProjectsDeploymentAlert v-if="store.environment" />

      <div class="flex-1 bg-default px-6 py-6 lg:px-8">
        <slot />
      </div>
    </div>
  </div>
</template>
