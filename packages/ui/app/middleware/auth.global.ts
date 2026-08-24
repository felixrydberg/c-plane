import useStore from "~/stores/store";

export default defineNuxtRouteMiddleware(async (to, from) => {
  const store = useStore();

  const publicPages = ["/auth"];
  const isPublicPage = publicPages.some(page => {
    const matches = to.path.startsWith(page);
    return matches;
  });

  if (isPublicPage) {
    if (to.path.startsWith("/auth")) {
      if (store.session || store.user) {
        const redirectPath = store.organization?.slug ? `/${store.organization.slug}` : '/organization/create';

        if (to.path === from.path) {
          return navigateTo(redirectPath);
        } else {
          return abortNavigation();
        }
      }
    }
    return;
  }

  if (store.session && store.user && !store.user.name?.trim() && to.path !== '/onboarding/username') {
    return navigateTo('/onboarding/username');
  }

  const organization = store.organization;
  const organizationOnboardingPath = organization?.slug && `/${organization.slug}/onboarding`;
  if (store.session && store.user && organization && organizationOnboardingPath && !store.projects.length && to.path !== organizationOnboardingPath) {
    const organizationId = organization.id
    const { data } = await useCplaneFetch<typeof store.projects>(`/api/organization/${organizationId as ':organization_id'}/projects` as const, {
      default: () => [],
    })
    if (store.organization?.id !== organizationId) return
    store.projects = data.value ?? [];

    if (!store.projects.length) {
      return navigateTo(organizationOnboardingPath);
    }
  }

  if (!store.session && !store.user) {
    return navigateTo('/auth/signin?redirect=' + encodeURIComponent(to.fullPath));
  }

  const isOrgSettingsOwnerPage = /^\/[^/]+\/settings$/.test(to.path)
    || /^\/[^/]+\/settings\/authentication/.test(to.path);
  if (isOrgSettingsOwnerPage && store.organization?.member?.role !== 'owner') {
    const slug = store.organization?.slug;
    return navigateTo(slug ? `/${slug}/settings/members` : '/organization/create');
  }
});
