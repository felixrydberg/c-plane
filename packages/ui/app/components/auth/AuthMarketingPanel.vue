<template>
  <div class="relative hidden overflow-hidden p-6 lg:block xl:p-8">
    <AuthHero />

    <div class="relative z-10 flex h-full flex-col justify-between px-10 py-12 xl:px-14 xl:py-14 text-white">
      <div class="space-y-16">
        <p class="text-lg font-semibold tracking-tight">
          CPlane
        </p>

        <div class="max-w-2xl space-y-6 font-space-mono">
          <h1 class="text-2xl font-semibold leading-none tracking-tight xl:text-5xl">
            Move work forward,
            <br>
            faster
          </h1>
          <p class="max-w-2xl text-lg leading-8 text-muted">
            The opinionated Kubernetes based cloud platform for high-velocity teams.
          </p>
        </div>
      </div>

      <div class="flex items-center gap-4 text-sm text-muted">
        <span></span>
        <span class="h-1 w-1 rounded-full" />
        <span></span>
      </div>
    </div>
  </div>
</template>
