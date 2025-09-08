import type { LoginFlow } from "@ory/client";

export default async () => {
  const { $ory } = useNuxtApp();
  const router = useRouter();
  const flow = ref<LoginFlow | null>(null);
  if (import.meta.client) {
    const result = (await $ory.createBrowserRegistrationFlow()).data
    router.replace({
      query: { flow: result.id }
    });
    flow.value = result;
  }
  return flow;
};
