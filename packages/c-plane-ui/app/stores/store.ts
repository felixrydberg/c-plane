import { defineStore } from "pinia";
import type { Identity, Session } from "~/types";

type State = {
  identity: Identity | null;
  session: Session | null;
}

const useStore = defineStore("store", {
  state: () => ({
    identity: null,
    session: null
  }) as State,
  getters: {
    
  },
  actions: {
    
  }
});
export default useStore;
