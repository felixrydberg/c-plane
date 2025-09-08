<script setup lang="ts">
  import * as z from 'zod'
  import type { FormSubmitEvent } from '@nuxt/ui'
  import type { UiNodeInputAttributes } from '@ory/client'

  const fields = reactive([{
    name: 'email',
    type: 'text' as const,
    label: 'Email',
    placeholder: 'Enter your email',
    required: true,
    error: null as string | null
  }, {
    name: 'password',
    label: 'Password',
    type: 'password' as const,
    placeholder: 'Enter your password',
    required: true,
    error: null as string | null
  }])

  const toast = useToast();
  const schema = z.object({
    email: z.email('Invalid email'),
    password: z.string().min(8, 'Must be at least 8 characters')
  })

  type Schema = z.output<typeof schema>
  const flow = await createLoginFlow();

  const onSubmit = async (payload: FormSubmitEvent<Schema>) => {
    const { $ory } = useNuxtApp();
    if (!flow.value) {
      throw createError("Login flow not initialized");
    }

    const formData = new FormData();
    const attributes = flow.value.ui.nodes.filter(node => node.type === 'input').map(node => node.attributes as UiNodeInputAttributes);
    const csrf_token = attributes.find(attr => attr.name === 'csrf_token')?.value;

    if (!csrf_token) {
      throw createError("CSRF token not found in the login flow");
    }

    formData.append('csrf_token', csrf_token);
    formData.append('identifier', payload.data.email);
    formData.append('password', payload.data.password);

    console.log(formData);
  }
</script>

<template>
  <UContainer class="flex justify-center items-center h-[100dvh]">
    <UPageCard class="w-full max-w-md">
      <UAuthForm
        :schema="schema"
        title="Sign in"
        description="Enter your credentials to access your account."
        :fields="fields"
        :loading="loading"
        @submit="onSubmit"
      />

    </UPageCard>
  </UContainer>
</template>
