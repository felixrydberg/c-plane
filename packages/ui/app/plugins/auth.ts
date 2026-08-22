import useStore from "~/stores/store"
import { getSession, loadProjectEnvironments } from "~/utils/auth"

async function fetchProjects() {
  const store = useStore()
  if (!store.organization?.id) return

  const { data } = await cplaneFetch(`/api/organization/${store.organization.id as ':organization_id'}/projects` as const)
  store.projects = data ?? []
}

async function syncCurrentProjectEnvironments() {
  const store = useStore()
  const route = useRoute()
  const projectId = route.params.project_id as string | undefined
  if (!projectId) return

  const project = store.projects.find(project => project.id === projectId)
  if (!project) return

  const environmentId = route.params.environment_id as string | undefined
  if (store.environments_project_id === project.id) {
    store.$patch({
      project,
      environment: store.environments.find(environment => environment.id === environmentId) ?? store.environments.find(environment => environment.is_default) ?? store.environments[0] ?? null,
    })
    return
  }

  await loadProjectEnvironments(project.id, environmentId)
}

export default defineNuxtPlugin(async () => {
  const store = useStore();
  if (import.meta.server && store.session === null) {
    await getSession();
    await fetchProjects();
    await syncCurrentProjectEnvironments();
  }

  if (import.meta.client) {
    const route = useRoute()
    watch([() => route.params.project_id, () => route.params.environment_id, () => store.projects], () => void syncCurrentProjectEnvironments(), { immediate: true })
  }
})
