<template>
  <UApp :toaster="null">
    <NuxtLayout>
      <NuxtPage />
    </NuxtLayout>
    <ClientOnly>
      <UToaster />
    </ClientOnly>
  </UApp>
</template>

<style>
.auth-to-dashboard-enter-active,
.auth-to-dashboard-leave-active {
  will-change: opacity, transform;
  transition: opacity 420ms cubic-bezier(0.23, 1, 0.32, 1), transform 420ms cubic-bezier(0.23, 1, 0.32, 1);
}

.auth-to-dashboard-enter-from,
.auth-to-dashboard-leave-to {
  opacity: 0;
  transform: scale(0.985);
}

@media (prefers-reduced-motion: reduce) {
  .auth-to-dashboard-enter-active,
  .auth-to-dashboard-leave-active {
    transition: opacity 120ms ease;
  }

  .auth-to-dashboard-enter-from,
  .auth-to-dashboard-leave-to {
    transform: none;
  }
}
</style>
