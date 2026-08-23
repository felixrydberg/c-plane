<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'

defineProps<{
  collapsed?: boolean
}>();

const store = useStore();
const router = useRouter();
const isDropdownOpen = ref(false);

const { data: organizations, pending, refresh } = useLazyFetch('/ui-api/organization');
type Organization = NonNullable<typeof organizations.value>['data'][number];

const items = computed<DropdownMenuItem[][]>(() => {
  const organization = store.organization;
  const organizationData = ((organizations.value as { data: Organization[] } | null)?.data ?? []);

  if ((!organization || organization === null) || !organizationData.length) {
    return [[{
      label: 'No organizations available',
      disabled: true,
    }], [{
      label: 'Create Organization',
      icon: 'i-heroicons:plus',
      onSelect() {
        router.push('/organization/create');
      }
    }]];
  }

  const orgs = organizationData.map((org: Organization) => {
    if (organization && org.id === organization.id) {
      return organization;
    }
    return org;
  }) || [organization]; 

  const orgItems = pending.value
    ? [{
        label: 'Loading organizations...',
        disabled: true,
      }]
    : orgs.map((org: Organization) => ({
    label: org.name,
    avatar: {
      src: org.logo || undefined,
      alt: org.name || 'Organization Avatar',
    },
    value: org.id,
    async onSelect() {
      if (store.organization?.id === org.id) {
        return;
      }
      await setOrganization(org.id, `/${org.slug}`);
    }
  }));

  return [
    orgItems,
    [{
      label: 'Create Organization',
      icon: 'i-heroicons:plus',
      onSelect() {
        router.push('/organization/create');
      }
    }, {
      label: 'Manage Organizations',
      icon: 'i-heroicons:cog-6-tooth',
      onSelect() {
        router.push('/settings/organizations');
      }
    }]
  ];
});

const handleDropdownOpen = () => {
  if (isDropdownOpen.value && !organizations.value) {
    refresh();
  }
};

watch(isDropdownOpen, (newValue) => {
  if (newValue) {
    handleDropdownOpen();
  }
});
</script>

<template>
  <UDropdownMenu
    :items="items"
    :content="{ align: 'center', collisionPadding: 12 }"
    :ui="{ content: collapsed ? 'w-40' : 'w-(--reka-dropdown-menu-trigger-width)' }"
    @open="isDropdownOpen = true"
    @close="isDropdownOpen = false"
  >
    <UButton
      v-bind="{
        avatar: {
          alt: store.organization?.name,
        },
        label: collapsed ? undefined : store.organization?.name,
        trailingIcon: collapsed ? undefined : 'i-lucide-chevrons-up-down'
      }"
      color="neutral"
      variant="soft"
      block
      :square="collapsed"
      class="data-[state=open]:bg-elevated mt-4"
      :class="[!collapsed && 'py-2']"
      :ui="{
        trailingIcon: 'text-dimmed'
      }"
    />
  </UDropdownMenu>
</template>
