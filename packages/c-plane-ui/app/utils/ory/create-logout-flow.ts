import useStore from "~/stores/store"

export default async () => {
  const { $ory } = useNuxtApp();
  const router = useRouter();
  const store = useStore();
  const flow = await $ory.createBrowserLogoutFlow();
  const update = await $ory.updateLogoutFlow({
    token: flow.data.logout_token,
  });
  store.session = null;
  store.identity = null;
  await router.push("/auth/signin");
}
