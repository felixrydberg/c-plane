<script setup lang="ts">
definePageMeta({
  redirect: (to) => {
    const project = to.params.project_id ? `/${to.params.project_id}` : ''
    return `/${to.params.organization_slug}/databases/stateful${project}`;
  },
});
</script>
