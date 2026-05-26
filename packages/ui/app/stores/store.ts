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
  }),

  actions: {}
})

export default useStore
