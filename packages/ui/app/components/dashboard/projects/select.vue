<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

const store = useStore();
const router = useRouter();
const route = useRoute();
const createModalOpen = ref(false);
const deleteModalOpen = ref(false);
const isDropdownOpen = ref(false);
const pending = ref(true);

async function fetchProjects() {
  if (!store.organization?.id) return;
  pending.value = true;
  try {
    const response = await $fetch(`/api/backend/organization/${store.organization.id}/projects`, {
      method: 'GET',
    });
    store.projects = (response?.data ?? []).map((p: { id: string; organization_id: string; name: string; default_branch_id: string | null }) => ({
      id: p.id,
      organization_id: p.organization_id,
      name: p.name,
      default_branch_id: p.default_branch_id,
    }));
  } catch {
    store.projects = [];
  } finally {
    pending.value = false;
  }
}

onMounted(() => { fetchProjects(); });

function navigateToProject(projectId: string | null) {
  const slug = store.organization?.slug;
  if (!slug) return;

  const currentProjectId = route.params.project_id as string | undefined;
  if (projectId === currentProjectId) return;

  // Build the path: /{slug}/{section}/{project_id}[/{branch_id}]
  const pathAfterSlug = route.path.slice(slug.length + 1); // e.g. "containers/proj-123/branch-456"

  if (projectId) {
    // Find the base section (everything before the current project_id)
    const baseSection = currentProjectId
      ? pathAfterSlug.slice(0, pathAfterSlug.indexOf(currentProjectId) - 1) // "containers"
      : pathAfterSlug.replace(/\/$/, ''); // "containers"

    router.push(`/${slug}/${baseSection}/${projectId}`);
    store.project = store.projects.find(p => p.id === projectId) || null;
  } else {
    // Navigate to the base section without project_id
    const baseSection = currentProjectId
      ? pathAfterSlug.slice(0, pathAfterSlug.indexOf(currentProjectId) - 1)
      : pathAfterSlug.replace(/\/$/, '');
    router.push(`/${slug}/${baseSection}`);
    store.project = null;
    store.branch = null;
    store.branches = [];
  }
}

const label = computed(() => store.project?.name || 'All Projects');

const items = computed<DropdownMenuItem[][]>(() => {
  if (pending.value) {
    return [[{
      label: 'Loading projects...',
      disabled: true,
    }]];
  }

  if (!store.projects.length) {
    return [[{
      label: 'No projects available',
      disabled: true,
    }], [{
      label: 'Create Project',
      icon: ICONS.folderPlus,
      onSelect() {
        createModalOpen.value = true;
      },
    }]];
  }

  const projectItems: DropdownMenuItem[] = [
    {
      label: 'All Projects',
      icon: ICONS.globeAlt,
      onSelect() { navigateToProject(null); },
    },
  ];

  for (const p of store.projects) {
    projectItems.push({
      label: p.name,
      icon: ICONS.folder,
      onSelect() { navigateToProject(p.id); },
    });
  }

  const actions: DropdownMenuItem[] = [{
    label: 'Create Project',
    icon: ICONS.folderPlus,
    onSelect() {
      createModalOpen.value = true;
    },
  }];

  if (store.project) {
    actions.push({
      label: 'Delete Project',
      icon: 'i-heroicons:trash',
      color: 'error' as const,
      onSelect() {
        deleteModalOpen.value = true;
      },
    });
  }

  return [
    projectItems,
    actions,
  ];
});

watch(isDropdownOpen, (open) => {
  if (open) fetchProjects();
});
</script>

<template>
  <UDropdownMenu
    :items="items"
    :content="{ align: 'start', collisionPadding: 12 }"
    :ui="{ content: 'w-64' }"
    @open="isDropdownOpen = true"
    @close="isDropdownOpen = false"
  >
    <UButton
      :label="label"
      :trailing-icon="ICONS.chevronUpDown"
      color="neutral"
      variant="soft"
      class="data-[state=open]:bg-elevated"
      :ui="{ trailingIcon: 'text-dimmed' }"
    />
  </UDropdownMenu>

  <DashboardProjectsCreateModal v-model:open="createModalOpen" @created="fetchProjects" />
  <DashboardProjectsDeleteModal v-model:open="deleteModalOpen" @deleted="fetchProjects" />
</template>
