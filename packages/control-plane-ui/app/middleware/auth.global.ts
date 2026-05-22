import useStore from "~/stores/store";

export default defineNuxtRouteMiddleware(async (to) => {
  const store = useStore();
  const publicPages = ["/auth", "/checkout-complete", "/session", "/handoff"];
  const isPublicPage = publicPages.some(page => {
    const matches = to.path.startsWith(page);
    return matches;
  });

  if (isPublicPage) {
    if (to.path.startsWith("/auth")) {
      if (store.session || store.user) {
        return abortNavigation();
      }
    }
  } else {
    if (!store.session && !store.user) {
      return navigateTo('/auth/signin?redirect=' + encodeURIComponent(to.fullPath));
    }
  }
});
