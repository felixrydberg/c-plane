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
