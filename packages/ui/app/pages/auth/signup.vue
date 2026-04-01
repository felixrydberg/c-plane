<script setup lang="ts">
  import * as z from 'zod'
  import type { FormSubmitEvent } from '@nuxt/ui'
  import useStore from '~/stores/store'
  import { createClient } from '~/utils/auth'
  import { passwordConfirmationSchema } from '~/utils/validation'

  const store = useStore();
  const route = useRoute();
  const router = useRouter();
  const loading = ref(false);

  const toast = useToast();
  const schema = z.object({
    name: z.string().min(1, 'Name is required'),
    email: z.string().email('Invalid email'),
  }).merge(passwordConfirmationSchema)

  type Schema = z.output<typeof schema>

  const state = reactive({
    email: '',
    name: '',
    password: '',
    confirmPassword: ''
  })

  const getQueryValue = (value: unknown): string | undefined => {
    if (Array.isArray(value)) {
      return typeof value[0] === 'string' ? value[0] : undefined
    }

    return typeof value === 'string' ? value : undefined
  }

  const emailFromQuery = getQueryValue(route.query.email)
  if (emailFromQuery) {
    state.email = emailFromQuery
  }

  const authSwitchQuery = computed(() => {
    const params = new URLSearchParams()
    const email = getQueryValue(route.query.email)
    const redirect = getQueryValue(route.query.redirect)
    const redirectTo = getQueryValue(route.query.redirectTo)

    if (email) {
      params.set('email', email)
    }

    if (redirect) {
      params.set('redirect', redirect)
    }

    if (redirectTo) {
      params.set('redirectTo', redirectTo)
    }

    const query = params.toString()
    return query.length > 0 ? `?${query}` : ''
  })

  const onSubmit = async (payload: FormSubmitEvent<Schema>) => {
    const client = createClient();
    const { email, password, name } = payload.data;
    if (loading.value) {
      return
    }
    loading.value = true
    const { error } = await client.signUp.email({
      email: email,
      password: password,
      name: name,
    })
    
    if (error) {
      console.error("Sign up error:", error)
      switch (error.code) {
        case "USER_ALREADY_EXISTS":
          toast.add({
            title: 'Email already in use!',
            color: 'error',
            icon: 'i-heroicons:x-mark'
          })
          break
        case "PASSWORD_TOO_SHORT":
          toast.add({
            title: 'Password too short!',
            color: 'error',
            icon: 'i-heroicons:x-mark'
          })
          break
        case "PASSWORD_COMPROMISED":
          toast.add({
            title: 'Password compromised!',
            description: error.message || 'This password has been compromised in a data breach, please choose a different one.',
            color: 'error',
            icon: 'i-heroicons:x-mark'
          })
      }
      loading.value = false;
      return
    }

    await getSession(false);

    if (store.session && store.user) {
      toast.add({
        title: 'Welcome aboard!',
        description: 'You have successfully signed up.',
        color: 'success',
        icon: 'i-heroicons:check-circle'
      })

      const redirectTo = getQueryValue(route.query.redirectTo)
      const redirect = getQueryValue(route.query.redirect)
      const path = redirectTo ?? (redirect ? decodeURIComponent(redirect) : '/organization/create')

      if (path.startsWith('/api/')) {
        window.location.assign(path)
        return
      }

      await router.push(path)
    }

    loading.value = false
  }
</script>

<template>
  <div class="space-y-8">
    <div class="space-y-6">
      <UiLogo size="lg" />

      <div class="space-y-2">
        <h1 class="text-3xl font-semibold tracking-tight sm:text-4xl">Sign up</h1>
        <p class="text-base text-muted">Enter your credentials to create a new account.</p>
      </div>
    </div>

    <UForm :schema="schema" :state="state" class="space-y-5" @submit.prevent="onSubmit">
      <UFormField label="Work email" name="email" required>
        <UInput
          v-model="state.email"
          type="text"
          placeholder="sarah@company.com"
          :disabled="loading"
          size="lg"
          class="w-full"
        />
      </UFormField>

      <UFormField label="Full name" name="name" required>
        <UInput
          v-model="state.name"
          type="text"
          placeholder="Enter your name"
          :disabled="loading"
          size="lg"
          class="w-full"
        />
      </UFormField>

      <UFormField label="Password" name="password" required>
        <UInput
          v-model="state.password"
          type="password"
          placeholder="Enter your password"
          :disabled="loading"
          size="lg"
          class="w-full"
        />
      </UFormField>

      <UFormField label="Confirm password" name="confirmPassword" required>
        <UInput
          v-model="state.confirmPassword"
          type="password"
          placeholder="Confirm your password"
          :disabled="loading"
          size="lg"
          class="w-full"
        />
      </UFormField>

      <UButton type="submit" block :loading="loading" size="lg" class="justify-center">
        Sign up
      </UButton>
    </UForm>

    <div class="mt-4 flex justify-center text-center text-sm text-muted">
      <p>
        Already have an account?
        <ULink :to="`/auth/signin${authSwitchQuery}`" as="span" class="ml-1 underline text-primary">
          Sign in here
        </ULink>
      </p>
    </div>
  </div>
</template>
