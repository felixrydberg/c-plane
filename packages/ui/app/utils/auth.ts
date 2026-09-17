import { createAuthClient } from "better-auth/vue"
import { inferAdditionalFields, twoFactorClient, adminClient, lastLoginMethodClient } from "better-auth/client/plugins"
import { passkeyClient } from "@better-auth/passkey/client"
import type { auth } from "~~/server/utils/auth"
import { useStore } from "~/stores/store"
import type { Environment, Project } from '@cplane/sdk'

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

type AuthContext = {
  store: ReturnType<typeof useStore>
  route: ReturnType<typeof useRoute>
  router: ReturnType<typeof useRouter>
  nuxtApp: ReturnType<typeof useNuxtApp>
  toast: ReturnType<typeof useToast>
  requestFetch: InternalFetch
}

const createAuthContext = (): AuthContext => ({
  store: useStore(),
  route: useRoute(),
  router: useRouter(),
  nuxtApp: useNuxtApp(),
  toast: useToast(),
  requestFetch: (import.meta.server ? useCplaneRequestFetch() : cplaneFetch) as unknown as InternalFetch,
})

async function loadProjectEnvironments(projectId: string, environmentId: string | undefined, context: AuthContext) {
  const { store, requestFetch } = context
  if (!store.organization?.id) return

  const project = store.projects.find(project => project.id === projectId)
  if (!project) return

  const environments = await requestFetch<Environment[]>(`/api/organization/${store.organization.id as ':organization_id'}/projects/${project.id as ':project_id'}/environments` as const)
  const environment = environments.find(environment => environment.id === environmentId) ?? environments.find(environment => environment.is_default) ?? environments[0] ?? null

  store.$patch({
    project,
    environments,
    environments_project_id: project.id,
    environment,
  })
}

const getSession = async (cache: boolean, context: AuthContext) => {
  const { store, route, router, nuxtApp, requestFetch } = context
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
        const orgResponse = await requestFetch<typeof store.organization>("/ui-api/organization/active", {
          method: "GET",
          ...(import.meta.client ? { credentials: "include" } : {}),
        });
        const organizationId = orgResponse?.id
        if (!organizationId) throw new Error('Active organization not found')
        store.setOrganization(orgResponse || null)
        const projectsResponse = await requestFetch<{ data?: Project[] }>(`/api/organization/${organizationId as ':organization_id'}/projects` as const, {
          ...(import.meta.client ? { credentials: "include" } : {}),
        });
        store.projects = projectsResponse.data ?? []
        const organizations = await requestFetch<{ data?: typeof store.organizations }>("/ui-api/organization", {
          method: "GET",
          ...(import.meta.client ? { credentials: "include" } : {}),
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

const createAuthError = (error: {
    code?: string | undefined;
    message?: string | undefined;
    status: number;
    statusText: string;
}, toast: ReturnType<typeof useToast>) => {
  toast.add({
    title: 'Authentication Error',
    description: error.message || error.statusText,
    color: 'error'
  })
};

const signOut = async (context: AuthContext) => {
    const { store, router } = context;
    const client = createClient();
    await client.signOut();
    // We have to set these to null so middleware will allow us to redirect
    store.session = null;
    store.user = null;
    await router.push('/auth/signin');
    store.$reset();
};

const setOrganization = async (id: string, redirect: string, context: AuthContext) => {
  const { store, router, requestFetch } = context;

  try {
    type OrgResponse = typeof store.organization & { projects?: Project[] };
    const data = await requestFetch<OrgResponse>(`/ui-api/organization/${id as ':organization_id'}`, {
      ...(import.meta.client ? { credentials: "include" } : {}),
    });

    if (!data) {
      throw new Error("Organization not found");
    }

    await requestFetch('/ui-api/organization/active', {
      method: 'POST',
      body: {
        organization_id: id
      }
    });
    
    store.setOrganization(data || null)
    store.projects = data?.projects ?? [];
    await router.push(redirect);
  } catch {
    throw new Error("Organization not found");
  }
}

export const useAuth = () => {
  const context = createAuthContext()

  return {
    createAuthError: (error: Parameters<typeof createAuthError>[0]) => createAuthError(error, context.toast),
    getSession: (cache = true) => getSession(cache, context),
    loadProjectEnvironments: (projectId: string, environmentId?: string) => loadProjectEnvironments(projectId, environmentId, context),
    setOrganization: (id: string, redirect = '/') => setOrganization(id, redirect, context),
    signOut: () => signOut(context),
  }
}
