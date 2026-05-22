type State = {
  user: User | null
  session: Session | null
}

export const useStore = defineStore("auth", {
  state: (): State => ({
    user: null,
    session: null,
  }),

  actions: {}
})

export default useStore
