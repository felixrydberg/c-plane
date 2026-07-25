<script setup lang="ts">
definePageMeta({
  layout: 'auth',
  layoutTransition: {
    name: 'auth-to-dashboard',
    mode: 'out-in',
  },
});

const store = useStore();
const route = useRoute();
const router = useRouter();

const organization_slug = computed(() => route.params.organization_slug?.toString());

if (!route.params.organization_slug) {
  try {
    const data = await $fetch("/api/organization/active", { method: "GET" });

    if (!data) {
      throw createError({ statusCode: 404, statusMessage: "Organization not found" });
    }

    const store = useStore();
    store.setOrganization(data);
    await router.push(`/${data.slug}`);
  } catch {
    throw createError({ statusCode: 404, statusMessage: "Organization not found" });
  }
} else {
  try {
    const slug = route.params.organization_slug.toString();
    const data = await $fetch("/api/organization/validate-slug", {
      method: "POST",
      body: { slug },
    });

    if (!data?.exists) {
      throw createError({ statusCode: 404, statusMessage: "Organization not found" });
    }

    if (!store.organization || store.organization.slug !== slug) {
      const { data: orgsResponse } = await useFetch<{ data: Organization[] }>(
        "/api/organization",
        { query: { search: slug } },
      );
      const matchedOrg = orgsResponse.value?.data?.find(organization => organization.slug === slug);
      if (!matchedOrg) {
        store.setOrganization(null);
        throw createError({ statusCode: 404, statusMessage: "Organization not found" });
      }

      await setOrganization(matchedOrg.id, route.fullPath);
    }
  } catch {
    throw createError({ statusCode: 404, statusMessage: "Organization not found" });
  }
}
</script>

<template>
  <NuxtPage :key="organization_slug" />
</template>
