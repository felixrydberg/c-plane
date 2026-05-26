<script setup lang="ts">
const open = defineModel<boolean>();

const props = defineProps<{
  organizationId?: string;
}>();

type OrganizationDetailsResponse = {
  organization: {
    id: string;
    name: string;
    email: string;
    slug: string;
    logo: string | null;
    created_at: string;
    polar_customer_id: string;
  };
  members: Array<{
    user_id: string;
    name: string | null;
    email: string;
    role: string;
    created_at: string;
  }>;
};

const NuxtTime = resolveComponent("NuxtTime");

const { data, status } = await useFetch<OrganizationDetailsResponse>(() => `/api/admin/organizations/${props.organizationId as ':organization_id'}`, {
  immediate: false,
  watch: [() => open.value, () => props.organizationId],
});

const organizationDetails = computed(() => data.value?.organization);
const members = computed(() => data.value?.members ?? []);
const isLoading = computed(() => status.value === 'pending');
</script>

<template>
  <UModal v-model:open="open" title="Organization Details" description="View organization information and membership summary">
    <template #body>
      <div class="space-y-4">
        <div v-if="isLoading" class="py-8 text-center">
          <UIcon name="i-lucide-loader-circle" class="size-5 text-muted animate-spin" />
        </div>

        <div v-else-if="organizationDetails" class="space-y-4">
          <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <div>
              <p class="text-xs text-muted tracking-wide">Name</p>
              <p class="text-sm font-medium">{{ organizationDetails.name }}</p>
            </div>
            <div>
              <p class="text-xs text-muted tracking-wide">Slug</p>
              <p class="text-sm font-medium">{{ organizationDetails.slug }}</p>
            </div>
            <div>
              <p class="text-xs text-muted tracking-wide">Email</p>
              <p class="text-sm font-medium">{{ organizationDetails.email }}</p>
            </div>
            <div>
              <p class="text-xs text-muted tracking-wide">Members</p>
              <p class="text-sm font-medium">{{ members.length }}</p>
            </div>
            <div>
              <p class="text-xs text-muted tracking-wide">Polar Customer</p>
              <p class="text-sm font-medium break-all">{{ organizationDetails.polar_customer_id }}</p>
            </div>
            <div>
              <p class="text-xs text-muted tracking-wide">Created At</p>
              <p class="text-sm font-medium">
                <NuxtTime
                  :datetime="organizationDetails.created_at"
                  locale="en"
                  date-style="short"
                  time-style="short"
                  :hour12="false"
                />
              </p>
            </div>
          </div>
        </div>
      </div>
    </template>
  </UModal>
</template>
