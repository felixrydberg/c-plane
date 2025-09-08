import useStore from "~/stores/store";

export default defineNuxtRouteMiddleware(async (to, from) => {
  const store = useStore();
  if (store.session || store.identity) {
    if (to.path === from.path) {
      return navigateTo('/');
    } else {
      return abortNavigation();
    }
  }
});
