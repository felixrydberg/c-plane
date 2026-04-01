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
        const redirectPath = store.organization?.slug ? `/${store.organization.slug}` : '/onboarding';

        if (to.path === from.path) {
          return navigateTo(redirectPath);
        } else {
          return abortNavigation();
        }
      }
    }
  } else {
    if (!store.session && !store.user) {
      return navigateTo('/auth/signin?redirect=' + encodeURIComponent(to.fullPath));
    }
  }
});
