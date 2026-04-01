<script setup lang="ts">
  const store = useStore();
  const router = useRouter();

  const links = [
    {
      label: "Back",
      icon: "i-heroicons:arrow-left",
      to: `/${store.organization?.slug}`,
      onSelect: () => {
        router.back()
      }
    },
    {
      label: "Settings",
      icon: "i-heroicons:adjustments-horizontal",
      open: true,
      children: [
        {
          label: "General",
          to: "/settings",
          exact: true,
        },
        {
          label: "Organizations",
          to: "/settings/organizations"
        },
      ]
    },
  ];
</script>

<template>
  <UDashboardGroup unit="rem" class="min-h-screen">
    <UDashboardSidebar
      class="pb-1 pt-8"
      :ui="{root:'border-none'}"
    >
      <template #default>
        <UNavigationMenu
          :items="links"
          orientation="vertical"
          tooltip
          popover
        />
      </template>
      <template #footer>
        <dashboard-user-overlay />
      </template>
    </UDashboardSidebar>
    <UDashboardPanel id="account-settings">
      <template #body>
        <div class="flex h-full items-center justify-center pt-8">
          <UiPageContainer title="Account Settings" size="max-w-2xl">
            <template #header-right>
              <UButton variant="link" class="text-muted mr-0 flex justify-center underline" @click="router.back()">
                Back
              </UButton>
            </template>
            <div class="flex flex-col gap-4 sm:gap-6 lg:gap-12 w-full max-w-2xl mx-auto">
              <NuxtPage />
            </div>
          </UiPageContainer>
        </div>
      </template>
    </UDashboardPanel>
  </UDashboardGroup>
</template>
