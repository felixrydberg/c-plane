import { defineNuxtPlugin } from "#app";
import { Configuration, FrontendApi } from "@ory/client";
import useStore from "~/stores/store";

const createClient = () => {
  const request_url = useRequestURL();
  return new FrontendApi(
    new Configuration({
      basePath: `${request_url.origin}/ory/kratos/`,
      baseOptions: {
        withCredentials: true,
        headers: {
          "Accept": "application/json",
        }
      },
    })
  );
};

export default defineNuxtPlugin(async () => {
  const store = useStore();
  const ory = createClient();
  if (import.meta.server) {
    try {
      const session = await validateServerSession(ory);
      if (session?.status === 200) {
        store.session = session.data;
        if (session.data.identity) {
          store.identity = session.data.identity;
        }
      }
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    } catch (_e) {
      // No valid session
    }
  }

  return {
    provide: {
      ory,
    },
  };
});

declare module '#app' {
  interface NuxtApp {
    ory: FrontendApi
  }
}
