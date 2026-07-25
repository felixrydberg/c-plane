<script setup lang="ts">
  import { refDebounced } from '@vueuse/core'
  
  const store = useStore();
  const toast = useToast();
  const router = useRouter();
  const query = ref("");
  const queryDebounced = refDebounced(query, 200);
  const limit = ref(50);
  const page = ref(1);
  const offset = computed(() => (page.value - 1) * limit.value);
  const { data, refresh } = await useFetch("/api/user/invitations", {
    query: {
      search: queryDebounced,
      limit: limit,
      offset: offset,
      status: "pending",
    },
  });

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const emits = defineEmits(['accepted', 'declined']);
  type Invitation = NonNullable<typeof data.value>['data'][number];
  interface AcceptInvitationResponse {
    organization: Organization;
  }

  const onAcceptClick = async (invitation: Invitation) => {
    try {
      const result = await $fetch<AcceptInvitationResponse>(`/api/user/invitations/${invitation.id as ':invitation_id'}/accept`, {
        method: 'POST',
      });

      const organization = result.organization;
      if (!organization) {
        throw new Error("Organization not found");
      }
      
      toast.add({
        title: "Invitation accepted",
        description: `You have been added to ${organization.name}`,
        color: "success",
      });

      if (!store.organizations.some((org) => org.id === organization.id)) {
        store.organizations.push(organization);
      }

      store.setOrganization(organization);
      emits('accepted', invitation);
      router.push(`/${organization.slug}`);
    } catch (error) {
      toast.add({
        title: "Error accepting invitation",
        description: error instanceof Error ? error.message : "An error occurred",
        color: "error",
      });
    }
  };

  const onDeclineClick = async (invitation: Invitation) => {
    try {
      await $fetch(`/api/organization/${invitation.organization_id as ':organization_id'}/invitations/${invitation.id as ':invitation_id'}` as string, {
        method: 'PATCH' as never,
        body: { action: 'decline' },
        query: { organization_id: invitation.organization_id, invitation_id: invitation.id }
      });

      toast.add({
        title: "Invitation declined",
        color: "success",
      });
      emits('declined', invitation);
      refresh();
    } catch (error) {
      toast.add({
        title: "Error declining invitation",
        description: error instanceof Error ? error.message : "An error occurred",
        color: "error",
      });
    }
  };
</script>

<template>
  <div class="flex flex-col gap-4">
    <UInput
      v-model="query"
      class="w-full"
      placeholder="Search for invitations..."
      icon="i-heroicons-magnifying-glass-20-solid"
    />
    <div class="flex flex-col items-center gap-4">
      <template v-if="(data?.data.length || 0) > 0">
        <div
          v-for="invitation in data?.data"
          :key="invitation.id"
          class="w-full flex text-sm text-muted gap-4 justify-between"
        >
          <div class="flex items-center gap-4">
            <UAvatar :alt="invitation.organization.name" size="sm" />
            {{ invitation.organization.name }}
          </div>
          <div class="space-x-2">
            <UButton
              size="sm"
              color="success"
              variant="soft"
              @click="onAcceptClick(invitation)"
            >
              Accept
            </UButton>
            <UButton
              size="sm"
              color="error"
              variant="soft"
              @click="onDeclineClick(invitation)"
            >
              Decline
            </UButton>
          </div>
        </div>
      </template>
      <template v-else>
        <p class="text-muted text-sm my-8">No invitations found.</p>
      </template>
      <UPagination
        v-if="(data?.pagination.total || 0) > limit"
        v-model:page="page"
        :total="data?.pagination.total"
        :items-per-page="limit"
      />
    </div>
  </div>
</template>
