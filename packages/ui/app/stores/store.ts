import type { Environment, Project } from '@cplane/sdk'

type State = {
  user: User | null
  session: Session | null
  organization: Organization | null
  organizations: Organization[]
  project: Project | null
  projects: Project[]
  environment: Environment | null
  environments: Environment[]
  environments_project_id: string | null
  // ponytail: change counter watched by resource pages.
  // Bump when a repoint/delete invalidates cached lists.
  // Incremented instead of keyed to avoid coupling to specific fetch shapes.
  refreshKey: number
}

export const useStore = defineStore("auth", {
  state: (): State => ({
    user: null,
    session: null,
    organization: null,
    organizations: [],
    project: null,
    projects: [],
    environment: null,
    environments: [],
    environments_project_id: null,
    refreshKey: 0,
  }),

  actions: {}
})

export default useStore
