<script setup lang="ts">
  import * as z from 'zod'
  import type { FormSubmitEvent } from '@nuxt/ui'
  import type { RegistrationFlow, UiNodeInputAttributes } from '@ory/client'

  const loading = ref(false);
  const fields = reactive([{
    name: 'email',
    type: 'text' as const,
    label: 'Email',
    placeholder: 'Enter your email',
    required: true,
    error: null as string | null
  }, {
    name: 'first',
    label: 'Firstname',
    type: 'text' as const,
    placeholder: 'Enter your firstname',
    required: true,
    error: null as string | null
  }, {
    name: 'last',
    label: 'Lastname',
    type: 'text' as const,
    placeholder: 'Enter your lastname',
    required: true,
    error: null as string | null
  }, {
    name: 'password',
    label: 'Password',
    type: 'password' as const,
    placeholder: 'Enter your password',
    required: true,
    error: null as string | null
  }, {
    name: 'confirmPassword',
    label: 'Confirm Password',
    type: 'password' as const,
    placeholder: 'Confirm your password',
    required: true,
    error: null as string | null
  }])

  const toast = useToast();
  const schema = z.object({
    first: z.string().min(1, 'Firstname is required'),
    last: z.string().min(1, 'Lastname is required'),
    email: z.email('Invalid email'),
    password: z.string().min(8, 'Must be at least 8 characters'),
    confirmPassword: z.string().min(8, 'Must be at least 8 characters')
  })
    .refine((data) => data.confirmPassword === data.password, { message: "Passwords don't match", path: ['confirmPassword'] })

  type Schema = z.output<typeof schema>
  const flow = await createRegistrationFlow();
  
  const onSubmit = async (payload: FormSubmitEvent<Schema>) => {
    const { $ory } = useNuxtApp();
    if (!flow.value) {
      throw createError("Register flow not initialized");
    }

    const attributes = flow.value.ui.nodes.filter(node => node.type === 'input').map(node => node.attributes as UiNodeInputAttributes);
    const csrf_token = attributes.find(attr => attr.name === 'csrf_token')?.value;

    if (!csrf_token) {
      throw createError("CSRF token not found in the register flow");
    }

    loading.value = true;
    try {
      const result = await $ory.updateRegistrationFlow({
        flow: flow.value.id,
        updateRegistrationFlowBody: {
          csrf_token,
          password: payload.data.password,
          method: "password",
          traits: {
            email: payload.data.email,
            name: {
              first: payload.data.first,
              last: payload.data.last
            }
          }
        }
      })

      const { data } = result;
      const { identity, session } = data;

    } catch (err) {
      if (err && typeof err === 'object' && 'response' in err && typeof err.response === 'object' && err.response && 'data' in err.response) {
        const error = err.response.data as RegistrationFlow;
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
        title="Sign up"
        description="Enter your credentials to create a new account."
        :fields="fields"
        :loading="loading"
        @submit="onSubmit"
      />
    </UPageCard>
  </UContainer>
</template>
