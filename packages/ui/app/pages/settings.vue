<script setup lang="ts">
  const store = useStore();
  const router = useRouter();

  const navigationUi = {
    list: 'space-y-1',
    item: 'px-0',
    link: 'h-[34px] gap-3 overflow-hidden rounded-md px-3 py-0 font-medium text-default text-sm hover:before:bg-elevated',
    childItem: '[&>a]:font-medium',
    childLink: 'px-3 py-2 text-sm',
    linkLeadingIcon: 'size-5 text-dimmed',
    linkTrailingIcon: 'size-4 rotate-0 text-muted opacity-60 transition-transform duration-200 ease-out group-hover:text-default group-hover:opacity-100 group-data-[state=open]:rotate-90',
  }

  const links = [
    {
      label: "Back",
      icon: "i-heroicons:arrow-left",
      to: store.organization?.slug ? `/${store.organization.slug}` : '/organization/create',
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
      class="border-e border-default pb-1"
      :ui="{ body: 'px-2 py-2' }"
    >
      <template #default>
        <UNavigationMenu
          :items="links"
          orientation="vertical"
          tooltip
          popover
          :ui="navigationUi"
        />
      </template>
      <template #footer>
        <dashboard-user-overlay />
      </template>
    </UDashboardSidebar>
    <UDashboardPanel id="account-settings">
      <template #body>
        <div class="flex min-h-full items-start justify-center pt-8">
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
