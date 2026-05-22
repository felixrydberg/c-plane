import useStore from "~/stores/store"
import { getSession } from "~/utils/auth"

export default defineNuxtPlugin(async () => {
  const store = useStore();
  const route = useRoute();
  const publicPageWithoutAuth = ["/auth"];
  if (publicPageWithoutAuth.some(page => route.path.startsWith(page))) {
    return
  }

  if (import.meta.server && store.session === null) {
    const session = await getSession();
    if (!session) {
      return
    }
   
    store.session = session.session;
    store.user = session.user;
  }
})
