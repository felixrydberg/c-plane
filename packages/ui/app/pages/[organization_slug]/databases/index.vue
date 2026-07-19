<script setup lang="ts">
definePageMeta({
  redirect: (to) => {
    const project = to.params.project_id ? `/${to.params.project_id}` : ''
    return project ? `/${to.params.organization_slug}/databases/postgres${project}` : `/${to.params.organization_slug}`
  },
});
</script>
