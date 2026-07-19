type Project = {
  id: string
  organization_id: string
  name: string
  default_branch_id: string | null
}

type Branch = {
  id: string
  name: string
  timeline: string
  is_default: boolean
  has_recent_undeployed_revision: boolean
}

type State = {
  user: User | null
  session: Session | null
  organization: Organization | null
  organizations: Organization[]
  project: Project | null
  projects: Project[]
  branch: Branch | null
  branches: Branch[]
  branches_project_id: string | null
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
    branch: null,
    branches: [],
    branches_project_id: null,
    refreshKey: 0,
  }),

  actions: {}
})

export default useStore
