<script setup lang="ts">
import * as z from 'zod'
import type { FormSubmitEvent } from '@nuxt/ui'
import useStore from '~/stores/store'
import { createAuthError, createClient } from '~/utils/auth'
import { passwordSchema } from '~/utils/validation'

const store = useStore();
const router = useRouter();
const route = useRoute();
const loading = ref(false);
const resetLoading = ref(false);
const resetModalOpen = ref(false);
const showTwoFactor = ref(false);
const useBackupCode = ref(false);

const toast = useToast();
const schema = z.object({
  email: z.email('Invalid email'),
  password: passwordSchema
})

type Schema = z.output<typeof schema>
const state = reactive({
  email: '',
  password: ''
});
const passwordError = ref<string>();

const resetPasswordSchema = z.object({
  email: z.email('Invalid email')
})

type ResetPasswordSchema = z.output<typeof resetPasswordSchema>

const resetPasswordState = reactive<ResetPasswordSchema>({
  email: ''
})

const twoFactorSchema = z.object({
  code: z.array(z.string().length(1, 'Code must be 6 digits')).length(6, 'Code must be 6 digits')
})
type TwoFactorSchema = z.output<typeof twoFactorSchema>
const twoFactorState = reactive<TwoFactorSchema>({
  code: []
});
const twoFactorError = ref<string>();

const backupCodeSchema = z.object({
  code: z.string().min(6, 'Backup code is required')
})
type BackupCodeSchema = z.output<typeof backupCodeSchema>
const backupCodeState = reactive<BackupCodeSchema>({
  code: ''
});
const backupCodeError = ref<string>();

const onSubmit = async (payload: FormSubmitEvent<Schema>) => {
  const client = createClient();
  const { email, password } = payload.data;
  if (loading.value) {
    return
  }
  loading.value = true
  const { data, error } = await client.signIn.email({
    email: email,
    password: password
  });

  if (data && "twoFactorRedirect" in data) {
    showTwoFactor.value = true;
    loading.value = false;
    return;
  }

  if (error) {
    const creationError = error;
    if (creationError) {
      createAuthError(creationError);
      loading.value = false
    }
  } else {
    await onUserAuthenticated();
  }
}

const onTwoFactorSubmit = async () => {
  const client = createClient();
  if (loading.value) {
    return
  }
  loading.value = true;
  twoFactorError.value = undefined;

  const { error } = await client.twoFactor.verifyTotp({
    code: twoFactorState.code.join(''),
  });

  if (error) {
    twoFactorError.value = 'Invalid code. Please try again.';
    loading.value = false;
    return;
  }

  await onUserAuthenticated();
}

const onBackupCodeSubmit = async () => {
  const client = createClient();
  if (loading.value) {
    return
  }
  loading.value = true;
  backupCodeError.value = undefined;

  const { error } = await client.twoFactor.verifyBackupCode({
    code: backupCodeState.code,
  });

  if (error) {
    backupCodeError.value = 'Invalid backup code. Please try again.';
    loading.value = false;
    return;
  }

  await onUserAuthenticated();
}

const onUserAuthenticated = async () => {
  const client = createClient();
  const { data: session, error: sessionError } = await client.getSession();
  if (sessionError) {
    const error = sessionError
    if (error) {
      if (error.code === "INVALID_PASSWORD") {
        passwordError.value = 'The password you entered is incorrect.';
      } else {
        createAuthError(error);
      }
      loading.value = false
    }
  } else if (session) {
    toast.add({
      title: 'Success',
      description: 'Signed in to CPlane Admin.',
      icon: 'i-heroicons:check-circle',
      color: 'success',
    })
    store.user = session.user
    store.session = session.session
    const redirect = route.query.redirect as string | undefined;
    const path = redirect ? decodeURIComponent(redirect) : '/';
    await router.push(path);
  }
}

const onRequestPasswordReset = async (payload: FormSubmitEvent<ResetPasswordSchema>) => {
  const client = createClient();

  if (resetLoading.value) {
    return;
  }

  resetLoading.value = true;

  const redirectTo = `${window.location.origin}/auth/reset-password`;
  const { error } = await client.requestPasswordReset({
    email: payload.data.email,
    redirectTo,
  });

  if (error) {
    toast.add({
      title: 'An Error accured',
      description: error.message || 'Failed to send the password reset email.',
      color: 'error'
    });
    resetLoading.value = false;
    return;
  }

  toast.add({
    title: 'Success',
    description: 'If an account exists for that email, a password reset link has been sent.',
    color: 'success'
  });

  resetModalOpen.value = false;
  resetLoading.value = false;
}

const TOTPForm = useTemplateRef("TOTPForm");
</script>

<template>
  <div class="space-y-8">
    <div class="space-y-6">
      <UiLogo size="lg" />

      <div class="space-y-2">
        <h2 class="text-3xl font-semibold tracking-tight sm:text-4xl">
          {{ showTwoFactor ? 'Secure admin sign in' : 'Welcome back, admin' }}
        </h2>
        <p class="text-base text-muted">
          {{ showTwoFactor && !useBackupCode ? 'Enter the 6-digit code from your authenticator app to continue to CPlane Admin.' : showTwoFactor && useBackupCode ? 'Enter one of your backup codes to finish admin sign in.' : 'Sign in to access the CPlane Admin console.' }}
        </p>
      </div>
    </div>

    <UForm v-if="!showTwoFactor" :schema="schema" :state="state" class="space-y-5" @submit.prevent="onSubmit">
      <UFormField label="Admin email" name="email" required>
        <UInput
          v-model="state.email"
          type="text"
          placeholder="admin@company.com"
          :disabled="loading"
          size="lg"
          class="w-full"
        />
      </UFormField>

      <UFormField label="Password" name="password" required>
        <template #hint>
          <button type="button" class="cursor-pointer text-sm text-muted transition-colors hover:text-primary" @click="resetPasswordState.email = state.email; resetModalOpen = true">
            Forgot password?
          </button>
        </template>
        <UInput
          v-model="state.password"
          type="password"
          placeholder="Enter your password"
          :disabled="loading"
          class="w-full"
          size="lg"
          :error="passwordError"
        />
      </UFormField>

      <UButton type="submit" block :loading="loading" size="lg" class="justify-center">
        Sign in to Admin
      </UButton>
    </UForm>

    <UForm
      v-else-if="showTwoFactor && !useBackupCode"
      ref="TOTPForm"
      :schema="twoFactorSchema"
      :state="twoFactorState"
      class="space-y-5"
      :validate-on="['blur', 'change']"
      @submit.prevent="onTwoFactorSubmit"
    >
      <UFormField
        label="Authentication code"
        name="code"
        class="w-fit"
        :error="twoFactorError"
      >
        <UPinInput
          v-model="twoFactorState.code"
          :length="6"
          :disabled="loading"
          class="mx-auto"
          size="lg"
          @complete="TOTPForm?.submit()"
        />
      </UFormField>

      <UButton
        type="button"
        variant="soft"
        size="lg"
        block
        :disabled="loading"
        class="justify-center"
        @click="useBackupCode = true; backupCodeState.code = ''; backupCodeError = undefined"
      >
        Use a backup code instead
      </UButton>

      <div class="flex gap-3">
        <UButton
          type="button"
          variant="soft"
          block
          size="lg"
          :disabled="loading"
          class="justify-center"
          @click="showTwoFactor = false; twoFactorState.code = []; useBackupCode = false"
        >
          Back
        </UButton>
        <UButton type="submit" block :loading="loading" class="justify-center">
          Verify
        </UButton>
      </div>
    </UForm>

    <UForm
      v-else-if="showTwoFactor && useBackupCode"
      :schema="backupCodeSchema"
      :state="backupCodeState"
      class="space-y-5"
      @submit.prevent="onBackupCodeSubmit"
    >
      <UFormField label="Backup code" name="code" required :error="backupCodeError">
        <UInput
          v-model="backupCodeState.code"
          type="text"
          placeholder="Enter your backup code"
          :disabled="loading"
          size="lg"
          class="w-full"
        />
      </UFormField>

      <UButton
        type="button"
        variant="soft"
        size="lg"
        block
        :disabled="loading"
        class="justify-center"
        @click="useBackupCode = false; twoFactorState.code = []; twoFactorError = undefined"
      >
        Use an authenticator app
      </UButton>

      <div class="flex gap-3">
        <UButton
          type="button"
          variant="soft"
          block
          size="lg"
          :disabled="loading"
          class="justify-center"
          @click="showTwoFactor = false; backupCodeState.code = ''; useBackupCode = false"
        >
          Back
        </UButton>
        <UButton type="submit" block :loading="loading" class="justify-center">
          Verify
        </UButton>
      </div>
    </UForm>

    <UModal v-model:open="resetModalOpen" title="Reset password" description="Enter your admin email and we will send you a reset link.">
      <template #body>
        <div class="space-y-4">
          <UForm :schema="resetPasswordSchema" :state="resetPasswordState" class="space-y-4" @submit.prevent="onRequestPasswordReset">
            <UFormField label="Admin email" name="email" required>
              <UInput
                v-model="resetPasswordState.email"
                type="email"
                placeholder="admin@company.com"
                :disabled="resetLoading"
                class="w-full"
              />
            </UFormField>
  
            <div class="flex justify-end gap-2">
              <UButton type="button" variant="soft" :disabled="resetLoading" @click="resetModalOpen = false">
                Cancel
              </UButton>
              <UButton type="submit" :loading="resetLoading">
                Send reset link
              </UButton>
            </div>
          </UForm>
        </div>
      </template>
    </UModal>
  </div>
</template>
