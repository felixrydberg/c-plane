<script setup lang="ts">
const open = defineModel<boolean>();

const props = defineProps<{
  user?: string;
}>();

const NuxtTime = resolveComponent("NuxtTime");

const { data, status } = await useFetch(() => `/api/admin/users/${props.user as ':user_id'}`);

const userDetails = computed(() => data.value);
const isLoading = computed(() => status.value === 'pending');
</script>

<template>
  <UModal v-model:open="open" title="User Details" description="View user information and organization memberships">
    <template #body>
      <div class="space-y-4">
        <div v-if="isLoading" class="text-center py-8">
          <p class="text-sm text-muted">Loading user details...</p>
        </div>
        <div v-else-if="userDetails">
          <div class="grid grid-cols-2 gap-4">
            <div>
              <p class="text-xs text-muted tracking-wide">Name</p>
              <p class="text-sm font-medium">{{ userDetails.user.name }}</p>
            </div>
            <div>
              <p class="text-xs text-muted tracking-wide">Email</p>
              <p class="text-sm font-medium">{{ userDetails.user.email }}</p>
            </div>
            <div v-if="userDetails.user.banned">
              <p class="text-xs text-muted tracking-wide">Ban Status</p>
              <p class="text-sm font-medium text-error">Banned</p>
              <p v-if="userDetails.user.banReason" class="text-xs text-muted">{{ userDetails.user.banReason }}</p>
            </div>
          </div>

          <div v-if="userDetails.memberships.length > 0" class="mt-6 pt-6 border-t">
            <p class="text-xs text-muted tracking-wide mb-3">Organizations</p>
            <div class="space-y-2">
              <div v-for="membership in userDetails.memberships" :key="membership.organization_id" class="flex justify-between items-center p-3 bg-gray-50 rounded">
                <div>
                  <p class="text-sm font-medium">{{ membership.organization_name }}</p>
                  <p class="text-xs text-muted capitalize">{{ membership.role }}</p>
                </div>
                <p class="text-xs text-muted">
                  <NuxtTime
                    :datetime="membership.created_at"
                    locale="en"
                    date-style="short"
                  />
                </p>
              </div>
            </div>
          </div>
          <div v-else class="mt-6 pt-6 border-t">
            <p class="text-xs text-muted">Not a member of any organizations</p>
          </div>
        </div>
      </div>
    </template>
  </UModal>
</template>
