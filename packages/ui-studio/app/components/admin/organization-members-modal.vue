<script setup lang="ts">
const open = defineModel<boolean>();

const props = defineProps<{
  organizationId?: string;
}>();

type OrganizationMembersResponse = {
  organization: {
    id: string;
    name: string;
  };
  members: Array<{
    user_id: string;
    name: string | null;
    email: string;
    role: string;
    created_at: string;
  }>;
};

const toast = useToast();
const isKickingMemberId = ref<string | null>(null);

const { data, status, refresh } = await useFetch<OrganizationMembersResponse>(() => `/api/admin/organizations/${props.organizationId as ':organization_id'}`, {
  immediate: false,
  watch: [() => open.value, () => props.organizationId],
});

const organization = computed(() => data.value?.organization);
const members = computed(() => data.value?.members ?? []);
const isLoading = computed(() => status.value === 'pending');

const onKickMember = async (memberUserId: string, memberName: string | null) => {
  if (!props.organizationId || isKickingMemberId.value) {
    return;
  }

  isKickingMemberId.value = memberUserId;
  try {
    await $fetch(`/api/admin/organizations/${props.organizationId}/members/${memberUserId}`, {
      method: 'DELETE',
    });

    toast.add({
      title: 'Success',
      description: `${memberName || 'Member'} was removed from the organization.`,
      color: 'success',
    });

    await refresh();
  } catch (error) {
    toast.add({
      title: 'An Error accured',
      description: error instanceof Error ? error.message : 'Failed to remove member.',
      color: 'error',
    });
  } finally {
    isKickingMemberId.value = null;
  }
};
</script>

<template>
  <UModal v-model:open="open" title="Organization Members" description="View members and remove access when needed">
    <template #body>
      <div class="space-y-4">
        <div v-if="isLoading" class="py-8 text-center">
          <p class="text-sm text-muted">Loading members...</p>
        </div>

        <div v-else-if="organization" class="space-y-4">
          <p class="text-sm text-muted">
            {{ organization.name }} has {{ members.length }} member<span v-if="members.length !== 1">s</span>.
          </p>

          <div v-if="members.length > 0" class="space-y-2">
            <div
              v-for="member in members"
              :key="member.user_id"
              class="flex items-center justify-between rounded-lg border border-default p-3"
            >
              <div>
                <p class="text-sm font-medium">{{ member.name || 'Unnamed user' }}</p>
                <p class="text-xs text-muted">{{ member.email }} • {{ member.role }}</p>
              </div>

              <UButton
                color="error"
                variant="soft"
                :loading="isKickingMemberId === member.user_id"
                :disabled="isKickingMemberId !== null"
                @click="onKickMember(member.user_id, member.name)"
              >
                Kick
              </UButton>
            </div>
          </div>

          <p v-else class="text-sm text-muted">No members found for this organization.</p>
        </div>
      </div>
    </template>
  </UModal>
</template>
