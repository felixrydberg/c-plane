import useStore from "~/stores/store"
import { getSession, loadProjectBranches } from "~/utils/auth"

async function fetchProjects() {
  const store = useStore()
  if (!store.organization?.id) return

  const requestFetch = import.meta.server ? useRequestFetch() : $fetch

  const { data } = await requestFetch<{ data: Array<{ id: string; organization_id: string; name: string; default_branch_id: string | null }> }>(
    `/api/backend/organization/${store.organization.id}/projects`
  )
  if (data?.length) {
    store.projects = data
  }
}

async function syncCurrentProjectBranches() {
  const store = useStore()
  const route = useRoute()
  const projectId = route.params.project_id as string | undefined
  if (!projectId) return

  const project = store.projects.find(project => project.id === projectId)
  if (!project) return

  const branchId = route.params.branch_id as string | undefined
  if (store.branches_project_id === project.id) {
    store.project = project
    store.branch = store.branches.find(branch => branch.id === branchId) ?? store.branches.find(branch => branch.is_default) ?? store.branches[0] ?? null
    return
  }

  await loadProjectBranches(project.id, branchId)
}

export default defineNuxtPlugin(async () => {
  const store = useStore();
  if (import.meta.server && store.session === null) {
    await getSession();
    await fetchProjects();
    await syncCurrentProjectBranches();
  }

  if (import.meta.client) {
    const route = useRoute()
    watch([() => route.params.project_id, () => route.params.branch_id], () => void syncCurrentProjectBranches(), { immediate: true })
  }
})
