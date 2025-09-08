<script setup lang="ts">
  definePageMeta({
    middleware: 'prevent-auth',
    layout: 'auth'
  });
</script>

<template>
  <NuxtPage />
</template>