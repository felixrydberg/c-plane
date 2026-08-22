<script setup lang="ts">
defineOptions({ name: 'OrganizationExternalRegistriesPage' })

const store = useStore()
const organizationId = computed(() => store.organization?.id ?? '')
</script>

<template>
  <div class="mx-auto flex w-full max-w-375 flex-col gap-8">
    <div class="border-b border-default/60 pb-5">
      <UiPageEyebrow label="Integrations" />
      <h1 class="mt-1 text-2xl font-semibold">External Registries</h1>
      <p class="mt-2 max-w-2xl text-sm text-muted">
        Connect reusable credentials for private OCI registries used by your containers.
      </p>
    </div>

    <RegistryExternalRegistriesPanel v-if="organizationId" :organization-id="organizationId" />
  </div>
</template>
