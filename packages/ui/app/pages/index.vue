<script setup lang="ts">
definePageMeta({
  redirect: () => {
    const store = useStore()
    const slug = store.organization?.slug
    return slug ? `/${slug}` : '/organization/create'
  },
})
</script>

<template>
  <div />
</template>