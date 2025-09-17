<script setup lang="ts">
  import * as z from 'zod'
  import type { FormSubmitEvent, FormFieldProps } from '@nuxt/ui'
  import type { LoginFlow, UiNodeInputAttributes } from '@ory/client'

  type AuthFormField = FormFieldProps & {
    name: string
    type?: 'checkbox' | 'select' | 'password' | 'text' | 'otp' | 'email'
    placeholder?: string
  }

  const loading = ref(false);
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
  }] as AuthFormField[])

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

    const attributes = flow.value.ui.nodes.filter(node => node.type === 'input').map(node => node.attributes as UiNodeInputAttributes);
    const csrf_token = attributes.find(attr => attr.name === 'csrf_token')?.value;

    if (!csrf_token) {
      throw createError("CSRF token not found in the login flow");
    }

    loading.value = true;
    try {
      const result = await $ory.updateLoginFlow({
        flow: flow.value.id,
        updateLoginFlowBody: {
          csrf_token,
          password: payload.data.password,
          method: "password",
          identifier: payload.data.email
        }
      })

      const { data } = result;
      const { identity, session } = data;

    } catch (err) {
      if (err && typeof err === 'object' && 'response' in err && typeof err.response === 'object' && err.response && 'data' in err.response) {
        const error = err.response.data as LoginFlow;
        const nodes = error.ui.nodes.filter(node => node.type === 'input');
        for (let i = 0; i < nodes.length; i++) {
          const node = nodes[i];
          if (node?.messages && node.messages.length > 0) {
            const message = node.messages[0];
            if (!message) {
              continue;
            }
            const field = fields.find(f => f.name === (node.attributes as UiNodeInputAttributes).name);
            if (field) {
              field.error = message.text;
            }
            toast.add({
              title: 'Success',
              description: message?.text,
              color: 'success'
            })
          }
        }
      } else {
        toast.add({
          title: 'Error',
          description: 'An unexpected error occurred. Please try again later.',
          color: 'error'
        })
      }
    } finally {
      loading.value = false;
    }
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
