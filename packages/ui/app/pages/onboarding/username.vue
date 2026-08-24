<script setup lang="ts">
import * as z from 'zod'
import type { FormSubmitEvent } from '@nuxt/ui'
import useStore from '~/stores/store'
import { getSession } from '~/utils/auth'

const store = useStore()
const router = useRouter()
const route = useRoute()
const loading = ref(false)
const schema = z.object({ name: z.string().trim().min(1, 'Username is required') })
type Schema = z.output<typeof schema>
const state = reactive<Schema>({ name: '' })

const onSubmit = async (event: FormSubmitEvent<Schema>) => {
  loading.value = true
  try {
    const user = await $fetch<{ name: string }>('/ui-api/user/profile', { method: 'PATCH', body: { name: event.data.name } })
    if (store.user) store.user.name = user.name
    const session = await getSession(false)
    if (!session || !store.session || !store.user?.name?.trim() || route.path !== '/onboarding/username') return
    await router.push(store.organization?.slug ? `/${store.organization.slug}` : '/organization/create')
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <main class="flex min-h-dvh items-center justify-center bg-default px-6 py-10">
    <UForm :schema="schema" :state="state" class="w-full max-w-sm space-y-6" @submit.prevent="onSubmit">
      <div class="space-y-2">
        <UiLogo size="lg" />
        <h1 class="text-3xl font-semibold tracking-tight">Choose a username</h1>
        <p class="text-muted">This name is shown to your team.</p>
      </div>

      <UFormField label="Username" name="name" required>
        <UInput v-model="state.name" autofocus autocomplete="username" :disabled="loading" class="w-full" />
      </UFormField>

      <UButton type="submit" :loading="loading" block class="justify-center">
        Continue
      </UButton>
    </UForm>
  </main>
</template>
