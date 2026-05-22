<script setup lang="ts">
  definePageMeta({
    layout: 'none',
  });
</script>

<template>
  <div class="min-h-dvh text-highlighted">
    <div class="grid min-h-dvh lg:grid-cols-[minmax(0,1.1fr)_minmax(0,0.9fr)]">
      <AuthMarketingPanel />

      <div class="flex min-h-dvh items-center justify-center px-6 py-10 sm:px-8 lg:px-12 xl:px-16">
        <div class="w-full max-w-lg">
          <NuxtPage />
        </div>
      </div>
    </div>
  </div>
</template>
