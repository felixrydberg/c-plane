import { createAuthClient } from "better-auth/vue"
import { inferAdditionalFields, twoFactorClient, adminClient  } from "better-auth/client/plugins"
import type { auth } from "~~/server/utils/auth"
import { useStore } from "~/stores/store"

export const createClient = () => {
  const url = import.meta.server ? useRequestURL().origin : window.location.origin;
  
  const fetchOptions: Record<string, Record<string, string>> = {};
  if (import.meta.server) {
    fetchOptions.headers = useRequestHeaders();
  }

  return createAuthClient({
    baseURL: url,
    fetchOptions,
    plugins: [ 
      inferAdditionalFields<typeof auth>(),
      twoFactorClient(),
      adminClient(),
    ]
  })
}

export type ClientType = ReturnType<typeof createClient>

export const getSession = async (cache: boolean = true) => {
  const store = useStore();
  try {
    const client = createClient();
    const { data, error } = await client.getSession({
      query: {
        disableCookieCache: !cache
      }
    });
    if (error || !data) {
      return;
    }

    const { session, user } = data;
    store.session = session
    store.user = user
    return data
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  } catch (_err) {
    // Expected error. Do nothing.
  }
}

export const createAuthError = (error: {
    code?: string | undefined;
    message?: string | undefined;
    status: number;
    statusText: string;
}) => {
  const toast = useToast();

  toast.add({
    title: 'Authentication Error',
    description: error.message || error.statusText,
    color: 'error'
  })
};

export const signOut = async () => {
    const store = useStore();
    const router = useRouter();
    const client = createClient();
    await client.signOut();
    // We have to set these to null so middleware will allow us to redirect
    store.session = null;
    store.user = null;
    await router.push('/auth/signin');
    store.$reset();
};
