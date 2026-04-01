<script setup lang="ts">
  const nuxtApp = useNuxtApp();
  const store = useStore();
  const session = await getSession();
  if (session) {
    store.session = session.session;
    store.user = session.user;
  } else {
    await nuxtApp.runWithContext(() => navigateTo("/auth/signin"));
  }
</script>

<template>
  <div>
    🍆
  </div>
</template>
