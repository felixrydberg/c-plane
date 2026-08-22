import { createAuthClient } from "better-auth/vue"
import { inferAdditionalFields, twoFactorClient, adminClient, lastLoginMethodClient } from "better-auth/client/plugins"
import { passkeyClient } from "@better-auth/passkey/client"
import type { auth } from "~~/server/utils/auth"
import { useStore } from "~/stores/store"
import type { Project } from '@cplane/sdk'

export const createClient = () => {
  const url = import.meta.server ? useRequestURL().origin : window.location.origin;
  
  const fetchOptions: Record<string, Record<string, string>> = {};
  if (import.meta.server) {
    fetchOptions.headers = useRequestHeaders();
  }

  return createAuthClient({
    baseURL: url,
    basePath: "/ui-api/auth",
    fetchOptions,
    plugins: [ 
      inferAdditionalFields<typeof auth>(),
      twoFactorClient(),
      adminClient(),
      passkeyClient(),
      lastLoginMethodClient(),
    ]
  })
}

export type ClientType = ReturnType<typeof createClient>
type InternalFetch = <T = unknown>(url: string, options?: Record<string, unknown>) => Promise<T>;

export async function loadProjectEnvironments(projectId: string, environmentId?: string) {
  const store = useStore()
  if (!store.organization?.id) return

  const project = store.projects.find(project => project.id === projectId)
  if (!project) return

  const response = await cplaneFetch(`/api/organization/${store.organization.id as ':organization_id'}/projects/${project.id as ':project_id'}/environments` as const)
  const environments = response
  const environment = environments.find(environment => environment.id === environmentId) ?? environments.find(environment => environment.is_default) ?? environments[0] ?? null

  store.$patch({
    project,
    environments,
    environments_project_id: project.id,
    environment,
  })
}

export const getSession = async (cache: boolean = true) => {
  const store = useStore();
  const route = useRoute();
  const router = useRouter();
  const nuxtApp = useNuxtApp();
  const requestFetch: InternalFetch | null = import.meta.server
    ? (cplaneFetch as unknown as InternalFetch)
    : null;
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
    const currentUser = !user.name?.trim() && store.user?.id === user.id && store.user.name?.trim()
      ? { ...user, name: store.user.name.trim() }
      : user
    store.session = session
    store.user = currentUser

    if (!currentUser.name?.trim()) {
      if (route.path !== '/onboarding/username') {
        if (import.meta.server) {
          return await nuxtApp.runWithContext(() => navigateTo('/onboarding/username'));
        }

        await router.push('/onboarding/username');
      }

      return data
    }

    if (user) {
      try {
        const orgResponse = import.meta.server
          ? await requestFetch!<typeof store.organization>("/ui-api/organization/active", {
            method: "GET"
          })
          : await cplaneFetch("/ui-api/organization/active", {
            method: "GET",
            credentials: "include"
          });
        store.setOrganization(orgResponse || null)
        const organizations = import.meta.server
          ? await requestFetch!<{ data?: typeof store.organizations }>("/ui-api/organization", {
            method: "GET"
          })
          : await cplaneFetch<{ data?: typeof store.organizations }>("/ui-api/organization", {
            method: "GET",
            credentials: "include"
          });
        store.organizations = organizations.data || [];
      } catch {
        if (route.path !== '/organization/create') {
          if (import.meta.server) {
            return await nuxtApp.runWithContext(() => navigateTo('/organization/create'));
          }

          await router.push('/organization/create');
        }
      }
    }

    return data
  } catch {
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

export const setOrganization = async (id: string, redirect: string = '/') => {
  const store = useStore();
  const router = useRouter();
  const requestFetch: InternalFetch | null = import.meta.server
    ? (cplaneFetch as unknown as InternalFetch)
    : null;

  try {
    type OrgResponse = typeof store.organization & { projects?: Project[] };
    const data = import.meta.server
      ? await requestFetch!<OrgResponse>(`/ui-api/organization/${id as ':organization_id'}`)
      : await cplaneFetch<OrgResponse>(`/ui-api/organization/${id as ':organization_id'}`, {
        credentials: "include"
      });

    if (!data) {
      throw new Error("Organization not found");
    }

    if (import.meta.server) {
      await requestFetch!('/ui-api/organization/active', {
        method: 'POST',
        body: {
          organization_id: id
        }
      });
    } else {
      await cplaneFetch('/ui-api/organization/active', {
        method: 'POST',
        body: {
          organization_id: id
        }
      });
    }
    
    store.setOrganization(data || null)
    store.projects = data?.projects ?? [];
    await router.push(redirect);
  } catch {
    throw new Error("Organization not found");
  }
}
