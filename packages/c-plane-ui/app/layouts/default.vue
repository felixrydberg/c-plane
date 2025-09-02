<!-- eslint-disable vue/no-multiple-template-root -->
<template>
  <header class="fixed h-16 w-full">
    <UContainer class="h-full flex items-end justify-between">
    </UContainer>
  </header>
  <main class="min-h-[100dvh]">
    <slot />
  </main>
  <footer class="bg-neutral-800">
  </footer>
</template>
