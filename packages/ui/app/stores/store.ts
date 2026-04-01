type State = {
  user: User | null
  session: Session | null
  organization: Organization | null
  organizations: Organization[]
}

export const useStore = defineStore("auth", {
  state: (): State => ({
    user: null,
    session: null,
    organization: null,
    organizations: [],
  }),

  actions: {}
})

export default useStore
