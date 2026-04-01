import useStore from "~/stores/store"
import { getSession } from "~/utils/auth"

export default defineNuxtPlugin(async () => {
  const store = useStore();
  if (import.meta.server && store.session === null) {
    await getSession();
  }
})
