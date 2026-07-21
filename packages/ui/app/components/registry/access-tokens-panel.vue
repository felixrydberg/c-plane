<script setup lang="ts">
import { ICONS } from '~/utils/icons'

const props = defineProps<{ organizationId: string }>()
const route = useRoute()
const toast = useToast()
const endpoint = computed(() => `/api/cplane/organization/${props.organizationId as ':organization_id'}/registry/access-tokens` as const)
const { data: tokens, refresh } = await useFetch(endpoint, { default: () => [] })
const revokingId = ref('')

async function revoke(token: NonNullable<typeof tokens.value>[number]) {
  revokingId.value = token.id
  try {
    await $fetch(`/api/cplane/organization/${props.organizationId as ':organization_id'}/registry/access-tokens/${token.id as ':token_id'}` as const, { method: 'DELETE' })
    await refresh()
  } catch {
    toast.add({ title: 'Could not revoke token', color: 'error' })
  } finally {
    revokingId.value = ''
  }
}
</script>

<template>
  <section class="space-y-3">
    <div v-if="!tokens.length" class="flex flex-col items-center justify-center py-16 gap-3 text-center border border-dashed border-default rounded-lg">
      <UIcon :name="ICONS.authentication" class="size-10 text-muted" />
      <p class="text-muted">No registry access tokens for this organization.</p>
    </div>
    <div v-else class="overflow-hidden border border-default rounded-lg">
      <table class="w-full text-sm">
        <thead class="bg-elevated text-left"><tr><th class="p-3">Name</th><th class="p-3">Created</th><th class="p-3" /></tr></thead>
        <tbody>
          <tr v-for="token in tokens" :key="token.id" class="border-t border-default">
            <td class="p-3">{{ token.name }}</td>
            <td class="p-3">{{ new Date(token.created_at).toLocaleDateString() }}</td>
            <td class="p-3 text-right">
              <div class="flex justify-end gap-2">
                <UButton :icon="ICONS.pencil" color="neutral" variant="solid" size="sm" :to="`/${route.params.organization_slug}/registry/access-tokens/${token.id}`">Edit</UButton>
                <UButton :icon="ICONS.trash" color="error" size="sm" :loading="revokingId === token.id" @click="revoke(token)">Revoke</UButton>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
