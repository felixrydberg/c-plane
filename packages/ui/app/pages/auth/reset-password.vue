<script setup lang="ts">
import type * as z from 'zod'
import type { FormSubmitEvent } from '@nuxt/ui'
import { createClient } from '~/utils/auth'
import { passwordConfirmationSchema } from '~/utils/validation'

const route = useRoute();
const router = useRouter();
const loading = ref(false);
const toast = useToast();

const token = computed(() => typeof route.query.token === 'string' ? route.query.token : '');
const resetError = computed(() => route.query.error === 'INVALID_TOKEN');

const schema = passwordConfirmationSchema;

type Schema = z.output<typeof schema>

const state = reactive<Schema>({
  password: '',
  confirmPassword: ''
})

const onSubmit = async (payload: FormSubmitEvent<Schema>) => {
  const client = createClient();

  if (!token.value || loading.value) {
    return;
  }

  loading.value = true;

  const { error } = await client.resetPassword({
    token: token.value,
    newPassword: payload.data.password,
  });

  if (error) {
    toast.add({
      title: 'An Error accured',
      description: error.message || 'Failed to reset your password.',
      color: 'error'
    });
    loading.value = false;
    return;
  }

  toast.add({
    title: 'Success',
    description: 'Your password has been reset successfully. You can now sign in.',
    color: 'success'
  });

  await router.push('/auth/signin');
}
</script>

<template>
  <div class="space-y-8">
    <div class="space-y-6">
      <UiLogo size="lg" />

      <div class="space-y-2">
        <h1 class="text-3xl font-semibold tracking-tight sm:text-4xl">Reset password</h1>
        <p class="text-base text-muted">Choose a new password for your account.</p>
      </div>
    </div>

    <div v-if="resetError || !token" class="rounded-lg border border-error/30 bg-error/10 p-4 text-sm text-error">
      This password reset link is invalid or has expired.
    </div>

    <UForm v-else :schema="schema" :state="state" class="space-y-5" @submit.prevent="onSubmit">
      <UFormField label="New password" name="password" required>
        <UInput
          v-model="state.password"
          type="password"
          placeholder="Enter your new password"
          :disabled="loading"
          size="lg"
          class="w-full"
        />
      </UFormField>

      <UFormField label="Confirm password" name="confirmPassword" required>
        <UInput
          v-model="state.confirmPassword"
          type="password"
          placeholder="Confirm your new password"
          :disabled="loading"
          size="lg"
          class="w-full"
        />
      </UFormField>

      <UButton type="submit" block :loading="loading" size="lg" class="justify-center">
        Reset password
      </UButton>
    </UForm>

    <div class="text-center text-sm text-muted">
      <ULink to="/auth/signin" class="underline text-primary">
        Back to sign in
      </ULink>
    </div>
  </div>
</template>