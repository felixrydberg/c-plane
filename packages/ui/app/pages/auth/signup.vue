<script setup lang="ts">
  import * as z from 'zod'
  import type { FormSubmitEvent } from '@nuxt/ui'
  import useStore from '~/stores/store'
  import { createClient } from '~/utils/auth'
  import { passwordConfirmationSchema } from '~/utils/validation'
  import { getQueryValue, useAuthSwitchQuery } from '~/utils/query'
  import { ICONS } from '~/utils/icons'

  const store = useStore();
  const route = useRoute();
  const router = useRouter();
  const loading = ref(false);
  const passkeyLoading = ref(false)
  const signupStep = ref<'identity' | 'password'>('identity')

  const toast = useToast();
  const identitySchema = z.object({
    name: z.string().trim().min(1, 'Username is required').max(100, 'Username is too long'),
    email: z.string().trim().email('Enter a valid email address'),
  })
  type IdentitySchema = z.output<typeof identitySchema>

  const state = reactive({
    email: '',
    name: '',
    password: '',
    confirmPassword: ''
  })

  const emailFromQuery = getQueryValue(route.query.email)
  if (emailFromQuery) {
    state.email = emailFromQuery
  }

  const authSwitchQuery = useAuthSwitchQuery()

  const onUserSignedUp = async () => {
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
  }

  const onSubmit = async () => {
    const client = createClient();
    if (loading.value) {
      return
    }
    loading.value = true
    const { error } = await client.signUp.email({
      email: state.email,
      password: state.password,
      name: state.name,
    })
    
    if (error) {
      console.error("Sign up error:", error)
      switch (error.code) {
        case "USER_ALREADY_EXISTS":
          toast.add({
            title: 'Email already in use!',
            color: 'error',
            icon: 'i-heroicons:exclamation-circle'
          })
          break
        case "PASSWORD_TOO_SHORT":
          toast.add({
            title: 'Password too short!',
            color: 'error',
            icon: 'i-heroicons:exclamation-circle'
          })
          break
        case "PASSWORD_COMPROMISED":
          toast.add({
            title: 'Password compromised!',
            description: error.message || 'This password has been compromised in a data breach, please choose a different one.',
            color: 'error',
            icon: 'i-heroicons:exclamation-circle'
          })
      }
      loading.value = false;
      return
    }

    await onUserSignedUp()

    loading.value = false
  }

  const onIdentitySubmit = (payload: FormSubmitEvent<IdentitySchema>) => {
    state.name = payload.data.name
    state.email = payload.data.email
    signupStep.value = 'password'
  }

  const onPasskeySignUp = async () => {
    if (passkeyLoading.value) return

    passkeyLoading.value = true
    try {
      const { error } = await createClient().passkey.addPasskey({
        context: JSON.stringify({ name: state.name, email: state.email }),
      })

      if (error) {
        toast.add({ title: 'Could not create passkey', description: error.message, color: 'error' })
        return
      }

      await onUserSignedUp()
    } catch (error) {
      toast.add({
        title: 'Could not create passkey',
        description: error instanceof Error ? error.message : 'Please try again.',
        color: 'error',
      })
    } finally {
      passkeyLoading.value = false
    }
  }
</script>

<template>
  <div class="space-y-8">
    <div class="space-y-6">
      <UiLogo size="lg" />

      <div class="space-y-2">
        <h1 class="text-3xl font-semibold tracking-tight sm:text-4xl">Sign up</h1>
        <p class="text-base text-muted">Create your account with a username and email.</p>
      </div>
    </div>

    <UiContentTransition>
      <div v-if="signupStep === 'identity'" key="identity" class="space-y-5">
        <UForm :schema="identitySchema" :state="state" class="space-y-5" @submit.prevent="onIdentitySubmit">
          <UFormField label="Email" name="email" required>
            <UInput
              v-model="state.email"
              type="email"
              placeholder="you@company.com"
              :disabled="loading"
              size="lg"
              class="w-full"
            />
          </UFormField>

          <UFormField label="Username" name="name" required>
            <UInput
              v-model="state.name"
              type="text"
              placeholder="Choose a username"
              :disabled="loading"
              size="lg"
              class="w-full"
            />
          </UFormField>

          <UButton type="submit" block :loading="loading" size="lg" class="justify-center">
            Continue
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

      <UForm v-else key="password" :schema="passwordConfirmationSchema" :state="state" class="space-y-5" @submit.prevent="onSubmit">
        <UFormField label="Password" name="password" required>
          <UInput v-model="state.password" type="password" placeholder="Enter your password" :disabled="loading || passkeyLoading" size="lg" class="w-full" />
        </UFormField>
        <UFormField label="Confirm password" name="confirmPassword" required>
          <UInput v-model="state.confirmPassword" type="password" placeholder="Confirm your password" :disabled="loading || passkeyLoading" size="lg" class="w-full" />
        </UFormField>

        <UButton type="submit" block :loading="loading" :disabled="passkeyLoading" size="lg" class="justify-center">
          Sign up
        </UButton>
        <UButton type="button" :icon="ICONS.passkey" color="neutral" variant="solid" block :loading="passkeyLoading" :disabled="loading" size="lg" class="justify-center" @click="onPasskeySignUp">
          Save a passkey instead
        </UButton>
        <UButton type="button" color="neutral" variant="ghost" block :disabled="loading || passkeyLoading" size="lg" class="justify-center" @click="signupStep = 'identity'">
          Back
        </UButton>
      </UForm>
    </UiContentTransition>
  </div>
</template>
