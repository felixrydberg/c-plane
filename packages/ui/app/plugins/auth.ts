import useStore from "~/stores/store"
import { useAuth } from "~/utils/auth"

export default defineNuxtPlugin(async () => {
  const store = useStore()
  const route = useRoute()
  const auth = useAuth()

  async function syncCurrentProjectEnvironments() {
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

    await auth.loadProjectEnvironments(project.id, environmentId)
  }

  if (import.meta.server && store.session === null) {
    await auth.getSession();
    await syncCurrentProjectEnvironments();
  }

  if (import.meta.client) {
    const route = useRoute()
    const router = useRouter()
    let currentPath = router.currentRoute.value.path
    router.beforeEach((to) => {
      if (to.path === currentPath) return
      currentPath = to.path
      store.clearBreadcrumbs()
    })
    watch([() => route.params.project_id, () => route.params.environment_id, () => store.projects], () => void syncCurrentProjectEnvironments(), { immediate: true })
  }
})
