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
type InternalFetch = <T = unknown>(url: string, options?: Record<string, unknown>) => Promise<T>;

type Branch = {
  id: string
  name: string
  timeline: string
  is_default: boolean
  has_recent_undeployed_revision: boolean
}

export async function loadProjectBranches(projectId: string, branchId?: string) {
  const store = useStore()
  if (!store.organization?.id) return

  const project = store.projects.find(project => project.id === projectId)
  if (!project) return

  const requestFetch: InternalFetch = import.meta.server ? useRequestFetch() as InternalFetch : $fetch
  const response = await requestFetch<Branch[] | { data: Branch[] }>(
    `/api/backend/organization/${store.organization.id}/projects/${project.id}/branches`
  )
  const branches = Array.isArray(response) ? response : response.data
  const branch = branches.find(branch => branch.id === branchId) ?? branches.find(branch => branch.is_default) ?? branches[0] ?? null

  store.project = project
  store.branches = branches
  store.branches_project_id = project.id
  store.branch = branch
}

export const getSession = async (cache: boolean = true) => {
  const store = useStore();
  const route = useRoute();
  const router = useRouter();
  const nuxtApp = useNuxtApp();
  const requestFetch: InternalFetch | null = import.meta.server
    ? (useRequestFetch() as unknown as InternalFetch)
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
    store.session = session
    store.user = user

    if (user) {
      try {
        const orgResponse = import.meta.server
          ? await requestFetch!<typeof store.organization>("/api/organization/active", {
            method: "GET"
          })
          : await $fetch("/api/organization/active", {
            method: "GET",
            credentials: "include"
          });
        store.organization = orgResponse || null;
        const organizations = import.meta.server
          ? await requestFetch!<{ data?: typeof store.organizations }>("/api/organization", {
            method: "GET"
          })
          : await $fetch<{ data?: typeof store.organizations }>("/api/organization", {
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
    ? (useRequestFetch() as unknown as InternalFetch)
    : null;

  try {
    type OrgResponse = typeof store.organization & { projects?: Array<{ id: string; organization_id: string; name: string; default_branch_id: string | null }> };
    const data = import.meta.server
      ? await requestFetch!<OrgResponse>(`/api/organization/${id as ':organization_id'}`)
      : await $fetch<OrgResponse>(`/api/organization/${id as ':organization_id'}`, {
        credentials: "include"
      });

    if (!data) {
      throw new Error("Organization not found");
    }

    if (import.meta.server) {
      await requestFetch!('/api/organization/active', {
        method: 'POST',
        body: {
          organization_id: id
        }
      });
    } else {
      await $fetch('/api/organization/active', {
        method: 'POST',
        body: {
          organization_id: id
        }
      });
    }
    
    store.organization = data || null;
    store.project = null;
    store.projects = data?.projects ?? [];
    store.branch = null;
    store.branches = [];
    store.branches_project_id = null;
    await router.push(redirect);
  } catch {
    throw new Error("Organization not found");
  }
}
