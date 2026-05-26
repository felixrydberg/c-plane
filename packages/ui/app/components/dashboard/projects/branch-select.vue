<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import { ICONS } from '~/utils/icons'

const store = useStore();
const router = useRouter();
const route = useRoute();
const createModalOpen = ref(false);
const historyModalOpen = ref(false);
const isDropdownOpen = ref(false);
const pending = ref(false);

type BranchItem = {
  id: string
  name: string
  timeline: string
  is_default: boolean
}

function selectBranch(b: BranchItem) {
  store.branch = { id: b.id, name: b.name, timeline: b.timeline, is_default: b.is_default };

  const projectId = route.params.project_id as string | undefined;
  const slug = store.organization?.slug;
  if (!projectId || !slug) return;

  const currentBranchId = route.params.branch_id;
  if (b.id === currentBranchId) return;

  const pathAfterSlug = route.path.slice(slug.length + 1);
  const baseSection = pathAfterSlug.slice(0, pathAfterSlug.indexOf(projectId) - 1);
  router.push(`/${slug}/${baseSection}/${projectId}/${b.id}`);
}

async function fetchBranches() {
  if (!store.organization?.id || !store.project?.id) {
    store.branches = [];
    return;
  }
  pending.value = true;
  try {
    const data = await $fetch<BranchItem[]>(
      `/api/backend/organization/${store.organization.id}/projects/${store.project.id}/branches`
    );
    store.branches = data;

    const current = store.branch;
    if (current && !data.find(b => b.id === current.id)) {
      store.branch = data.find(b => b.is_default) ?? null;
    } else if (!current && data.length > 0) {
      store.branch = data.find(b => b.is_default) ?? null;
    }
  } catch {
    store.branches = [];
  } finally {
    pending.value = false;
  }
}

const label = computed(() => {
  if (!store.project) return 'Select branch';
  if (!store.branch) return store.branches.length ? 'Select branch' : 'No branches';
  return store.branch.name;
});

const items = computed<DropdownMenuItem[][]>(() => {
  if (!store.project) {
    return [[{ label: 'Select a project first', disabled: true }]];
  }

  if (pending.value) {
    return [[{ label: 'Loading branches...', disabled: true }]];
  }

  if (!store.branches.length) {
    return [[{ label: 'No branches', disabled: true }], [{
      label: 'Create Branch',
      icon: ICONS.folderPlus,
      onSelect() { createModalOpen.value = true; },
    }]];
  }

  const branchItems: DropdownMenuItem[] = store.branches.map(b => ({
    label: b.name + (b.is_default ? ' (default)' : ''),
    icon: ICONS.folder,
    onSelect() { selectBranch(b); },
  }));

  const actions: DropdownMenuItem[] = [{
    label: 'Create Branch',
    icon: ICONS.folderPlus,
    onSelect() { createModalOpen.value = true; },
  }];

  if (store.branch) {
    actions.push({
      label: 'Branch History',
      icon: 'i-heroicons:clock',
      onSelect() { historyModalOpen.value = true; },
    });
  }

  return [
    branchItems,
    actions,
  ];
});

watch(isDropdownOpen, (open) => {
  if (open) fetchBranches();
});

watch(() => store.project?.id, () => {
  fetchBranches();
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
      :disabled="!store.project"
      class="data-[state=open]:bg-elevated"
      :ui="{ trailingIcon: 'text-dimmed' }"
    />
  </UDropdownMenu>

  <DashboardProjectsCreateBranchModal v-model:open="createModalOpen" @created="fetchBranches" />
  <DashboardProjectsBranchHistoryModal v-model:open="historyModalOpen" @updated="fetchBranches" />
</template>
