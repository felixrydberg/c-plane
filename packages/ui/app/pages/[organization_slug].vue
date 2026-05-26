<script setup lang="ts">
definePageMeta({
  layout: 'auth',
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
    store.organization = data;
    store.project = null;
    store.projects = [];
    await router.push(`/${data.slug}`);
  } catch (error) {
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
      const url = String(import.meta.server ? useRequestURL().origin : window.location.origin);
      const orgsResponse = await $fetch<{ data: Array<{ id: string; name: string; slug: string }> }>(
        `${url}/api/organization`,
        { query: { search: slug } },
      );
      const matchedOrg = orgsResponse?.data?.find((o: any) => o.slug === slug);
      if (matchedOrg) {
        store.organization = matchedOrg as any;
      }
    }
  } catch (error) {
    throw createError({ statusCode: 404, statusMessage: "Organization not found" });
  }
}
</script>

<template>
  <NuxtPage :key="organization_slug" />
</template>
