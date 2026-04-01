<script setup lang="ts">
  definePageMeta({
    layout: 'auth',
  });

  const store = useStore();
  const route = useRoute();
  const router = useRouter();

  const organization = computed(() => store.organization);
  const organization_slug = computed(() => {
    return route.params.organization_slug?.toString();
  });

  if (!route.params.organization_slug) {
    try {
      const data = await $fetch("/api/organization/active", {
        method: "GET",
      });

      if (!data) {
        throw createError({
          statusCode: 404,
          statusMessage: "Organization not found",
        });
      }

      const store = useStore();
      store.organization = data;
      await router.push(`/${data.slug}`);
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    } catch (error) {
      throw createError({
        statusCode: 404,
        statusMessage: "Organization not found",
      });
    }
  } else {
    try {
      const data = await $fetch("/api/organization/validate-slug", {
        method: "POST",
        body: { slug: route.params.organization_slug },
      });
    
      if (!data?.exists) {
        throw createError({
          statusCode: 404,
          statusMessage: "Organization not found",
        });
      }
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    } catch (error) {
      throw createError({
        statusCode: 404,
        statusMessage: "Organization not found",
      });
    }
  }
</script>

<template>
  <NuxtPage :key="organization_slug" />
</template>
