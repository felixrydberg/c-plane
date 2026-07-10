import useStore from "~/stores/store"
import { getSession } from "~/utils/auth"

async function fetchProjects() {
  const store = useStore()
  if (!store.organization?.id) return

  const requestFetch: <T = unknown>(url: string, options?: Record<string, unknown>) => Promise<T> =
    import.meta.server ? useRequestFetch() as any : $fetch as any

  const { data } = await requestFetch<{ data: Array<{ id: string; organization_id: string; name: string; default_branch_id: string | null }> }>(
    `/api/backend/organization/${store.organization.id}/projects`
  )
  if (data?.length) {
    store.projects = data
  }
}

export default defineNuxtPlugin(async () => {
  const store = useStore();
  if (import.meta.server && store.session === null) {
    await getSession();
    await fetchProjects();
  }
})
