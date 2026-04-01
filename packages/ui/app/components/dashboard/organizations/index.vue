<script setup lang="ts">
  import { refDebounced } from '@vueuse/core'
  
  const router = useRouter();
  const store = useStore();
  const toast = useToast();
  const query = ref("");
  const queryDebounced = refDebounced(query, 200);
  const limit = ref(50);
  const page = ref(1);
  const offset = computed(() => (page.value - 1) * limit.value);
  const emits = defineEmits(['left', 'settings']);

  type OrganizationsResponse = {
    data: Array<{
      id: string;
      name: string;
      slug: string;
      created_at: string;
      logo: string | null;
      member: {
        id: string;
        role: string;
      };
      active: boolean;
    }>;
    pagination: {
      total: number;
      limit: number;
      offset: number;
    };
  };

  const { data, refresh, pending } = await useFetch<OrganizationsResponse>("/api/organization", {
    query: {
      search: queryDebounced,
      limit: limit,
      offset: offset,
    },
  });

  type Organization = NonNullable<typeof data.value>['data'][number];
  const onLeaveClick = async (organization: Organization) => {
    try {
      await $fetch(`/api/organization/${organization.id as ':organization_id'}/membership`, {
        method: 'DELETE',
        query: {
          organization_id: organization.id,
        },
      });

      toast.add({
        title: 'Organization left',
        description: `You left ${organization.name}`,
        color: 'success',
      });

      await refresh();
      emits('left');

      if (store.organization?.id !== organization.id) {
        return;
      }

      const organizations = await $fetch('/api/organization');
      store.organizations = organizations.data || [];

      const nextOrganization = organizations.data?.[0];
      if (nextOrganization) {
        await setOrganization(nextOrganization.id, `/${nextOrganization.slug}`);
        return;
      }

      store.organization = null;
      await router.push('/onboarding');
    } catch (error) {
      toast.add({
        title: 'Error leaving organization',
        description: error instanceof Error ? error.message : 'An unknown error occurred',
        color: 'error',
      });
    }
  }

  const onSettingsClick = async (organization: Organization) => {
    await setOrganization(organization.id, `/${organization.slug}/administration`);
    emits('settings');
  }
</script>

<template>
  <div class="mx-auto w-full max-w-2xl rounded-2xl border border-default/60 bg-default/30 p-5 md:p-6">
    <div class="mb-5 space-y-1 text-center">
      <p class="text-lg font-semibold">Your Organizations</p>
      <p class="text-sm text-muted">Manage workspace access and switch where you want to work.</p>
    </div>

    <UInput
      v-model="query"
      class="w-full"
      placeholder="Search organizations"
      icon="i-heroicons-magnifying-glass-20-solid"
    />

    <div class="mt-4 flex flex-col gap-2">
      <template v-if="pending">
        <div class="rounded-xl border border-default/70 p-4 text-sm text-muted">
          Loading organizations...
        </div>
      </template>

      <template v-else-if="data && data.data.length > 0">
        <div
          v-for="organization in data.data"
          :key="organization.id"
          class="flex items-center justify-between gap-3 rounded-xl border border-default/70 bg-default/60 p-3"
        >
          <div class="flex min-w-0 items-center gap-3">
            <UAvatar :alt="organization.name" size="sm" />
            <div class="min-w-0">
              <p class="truncate text-sm font-medium text-highlighted">{{ organization.name }}</p>
              <div class="flex items-center gap-2 text-xs text-muted">
                <span class="capitalize">{{ organization.member.role }}</span>
                <UBadge v-if="organization.active" color="primary" variant="soft" size="xs">Active</UBadge>
              </div>
            </div>
          </div>

          <div class="flex items-center gap-2">
            <UButton
              v-if="organization.member.role === 'owner' || organization.member.role === 'admin'"
              size="xs"
              variant="soft"
              icon="i-heroicons:cog-6-tooth"
              @click="onSettingsClick(organization)"
            >
              Manage
            </UButton>
            <UButton
              v-else
              size="xs"
              variant="soft"
              color="error"
              @click="onLeaveClick(organization)"
            >
              Leave
            </UButton>
          </div>
        </div>
      </template>

      <template v-else>
        <div class="rounded-xl border border-dashed border-default/70 py-10 text-center">
          <p class="text-sm text-muted">No organizations found.</p>
        </div>
      </template>
    </div>

    <div class="mt-4 flex justify-center">
      <UPagination
        v-if="(data?.pagination.total || 0) > limit"
        v-model:page="page"
        :total="data?.pagination.total"
        :items-per-page="limit"
      />
    </div>
  </div>
</template>
