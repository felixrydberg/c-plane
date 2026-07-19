<script setup lang="ts">
import * as z from 'zod'
import useStore from '~/stores/store'
import { createAuthError, createClient } from '~/utils/auth';
import { useQRCode } from '@vueuse/integrations/useQRCode'

const store = useStore();
const toast = useToast();
const client = createClient();

const step = ref(0);
const stepperItems = [
  { title: 'Enter password' },
  { title: 'Scan QR Code' }
];

const twofaSchema = z.object({
  password: z.string().min(8, 'Password is required')
})
type TwoFASchema = z.output<typeof twofaSchema>
const twofaError = ref<string>();
const twofaState = reactive<TwoFASchema>({
  password: ''
})

const totpModalOpen = ref(false);
const totpURI = ref<string>('');
const qrcode = useQRCode(totpURI);

const totpForm = useTemplateRef("totpForm");
const totpSchema = z.object({
  code: z.array(z.string().length(1, 'Code must be 6 digits')).length(6, 'Code must be 6 digits')
})
type TOTPSchema = z.output<typeof totpSchema>
const totpState = reactive<TOTPSchema>({
  code: []
})

const backupCodesFromSetup = ref<string[]>([]);
const backupCodesModal = ref(false);
const backupCodesDisplay = ref<string[]>([]);
const backupCodesAcknowledgement = ref(false);

const showBackupCodes = (codes: string[]) => {
  backupCodesDisplay.value = codes;
  backupCodesAcknowledgement.value = false;
  backupCodesModal.value = true;
};

const closeBackupCodesModal = () => {
  backupCodesModal.value = false;
  backupCodesDisplay.value = [];
  backupCodesAcknowledgement.value = false;
};

const regenerate2FAModal = ref(false);
const Management2FASchema = z.object({
  password: z.string().min(8, 'Password is required')
});
type Management2FASchema = z.output<typeof Management2FASchema>;
const management2FAError = ref<string>();
const management2FAState = reactive<Management2FASchema>({
  password: ''
});

const disable2FAModal = ref(false);
const disable2FASchema = z.object({
  password: z.string().min(8, 'Password is required')
});
type Disable2FASchema = z.output<typeof disable2FASchema>;
const disable2FAError = ref<string>();
const disable2FAState = reactive<Disable2FASchema>({
  password: ''
});


const on2FAPasswordSubmit = async () => {
  const { data, error } = await client.twoFactor.enable({
    password: twofaState.password,
  });

  if (error) {
    if (error.code === "INVALID_PASSWORD") {
      twofaError.value = 'The password you entered is incorrect.';
    } else {
      twofaError.value = undefined;
      createAuthError(error);
    }
    return
  }

  backupCodesFromSetup.value = data.backupCodes;
  twofaState.password = '';
  totpURI.value = data.totpURI;
  step.value = 1;
}

const on2FACodeSubmit = async () => {
  const { error } = await client.twoFactor.verifyTotp({
    code: totpState.code.join(''),
  });

  if (error) {
    toast.add({
      title: 'Error verifying code. Please try again.',
      description: error.message,
      color: 'error'
    })
    return
  }

  if (!store.user) {
    toast.add({
      title: 'Error updating user state',
      description: 'User not found in store.',
      color: 'error'
    });
    return;
  }
  
  store.user.twoFactorEnabled = true;
  totpModalOpen.value = false;
  showBackupCodes(backupCodesFromSetup.value);
  
  toast.add({
    title: '2FA Enabled Successfully',
    description: 'Two-Factor Authentication has been enabled on your account.',
    color: 'success'
  });


}

const onTOTPModalClick = () => {
  totpModalOpen.value = true;
  totpState.code = [];
  totpURI.value = '';
  step.value = 0;
}

const onManagement2FASubmit = async () => {
  const { data, error } = await client.twoFactor.generateBackupCodes({
    password: management2FAState.password,
  });

  if (error) {
    if (error.code === "INVALID_PASSWORD") {
      management2FAError.value = 'The password you entered is incorrect.';
    } else {
      management2FAError.value = undefined;
      createAuthError(error);
    }
    return;
  }

  regenerate2FAModal.value = false;
  management2FAState.password = '';
  management2FAError.value = undefined;
  showBackupCodes(data.backupCodes);
  
  toast.add({
    title: 'Backup Codes Regenerated',
    description: 'New backup codes have been generated. Please save them securely.',
    color: 'success'
  });
}

const onDisable2FASubmit = async () => {
  const { error } = await client.twoFactor.disable({
    password: disable2FAState.password,
  });

  if (error) {
    if (error.code === "INVALID_PASSWORD") {
      disable2FAError.value = 'The password you entered is incorrect.';
    } else {
      disable2FAError.value = undefined;
      createAuthError(error);
    }
    return
  }

  toast.add({
    title: '2FA Disabled',
    description: 'Two-Factor Authentication has been disabled on your account.',
    color: 'neutral'
  });
  
  if (!store.user) {
    toast.add({
      title: 'Error updating user state',
      description: 'User not found in store.',
      color: 'error'
    });
    return;
  }
  
  disable2FAState.password = '';
  disable2FAModal.value = false;
  store.user.twoFactorEnabled = false;
}
</script>

<template>
  <div class="w-full border border-default rounded-lg p-6 space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <p class="font-semibold">Two-Factor Authentication</p>
        <p class="text-sm text-muted">Add an extra layer of security to your account.</p>
      </div>
    </div>

    <div class="flex items-center justify-between p-4 rounded-lg border border-dashed" :class="store.user?.twoFactorEnabled ? 'border-success/30' : 'border-error/30'">
      <template v-if="store.user?.twoFactorEnabled">
        <div class="flex items-center gap-3">
          <UIcon name="i-heroicons:shield-check" class="w-5 h-5 text-success" />
          <div>
            <p class="text-sm font-medium">Two-Factor Authentication Active</p>
            <p class="text-sm text-muted">Your account is protected</p>
          </div>
        </div>
        <UButton color="success" size="sm" variant="soft">
          Enabled
        </UButton>
      </template>
      <template v-else>
        <div class="flex items-center gap-3">
          <UIcon name="i-heroicons:shield-exclamation" class="w-5 h-5 text-error" />
          <div>
            <p class="text-sm font-medium">Enable Two-Factor Authentication</p>
            <p class="text-sm text-muted">Secure your account with TOTP</p>
          </div>
        </div>
        <UButton color="error" size="sm" variant="soft" @click="onTOTPModalClick">
          Enable
        </UButton>
        <UModal v-model:open="totpModalOpen" title="Enable 2FA" description="Set up two-factor authentication to enhance the security of your account.">
          <template #body>
            <UStepper v-model="step" :items="stepperItems" class="mb-4" />
            <template v-if="step === 0">
              <UForm :schema="twofaSchema" :state="twofaState" @submit.prevent="on2FAPasswordSubmit">
                <UFormField label="Password" :error="twofaError" description="Please enter your account password to proceed." name="password">
                  <UInput v-model="twofaState.password" type="password" class="w-full" placeholder="Enter your password" autocomplete="password" />
                </UFormField>
                <UButton class="w-full flex justify-center mt-4" type="submit">Continue</UButton>
              </UForm>
            </template>
            <template v-if="step === 1">
              <UForm ref="totpForm" :schema="totpSchema" :state="totpState" @submit.prevent="on2FACodeSubmit">
                <div class="mx-auto w-fit">
                  <UFormField label="Scan with your Authentication app">
                    <img :src="qrcode" alt="QR Code" class="mx-auto" />
                  </UFormField>
                  <UFormField label="Enter the code from your app">
                    <UPinInput v-model="totpState.code" class="mx-auto" otp :length="6" @complete="totpForm?.submit()" />
                  </UFormField>
                </div>
                <UButton class="w-full flex justify-center mt-4" type="submit" :disabled="totpState.code.length !== 6">Continue</UButton>
              </UForm>
            </template>
          </template>
        </UModal>
      </template>
    </div>

    <template v-if="store.user?.twoFactorEnabled">
      <USeparator />
      <div class="flex items-center justify-between p-4 rounded-lg border border-dashed border-default">
        <div class="flex items-center gap-3">
          <UIcon name="i-heroicons:key" class="w-5 h-5 text-muted" />
          <div>
            <p class="text-sm font-medium">Backup Codes</p>
            <p class="text-sm text-muted">Re-generate recovery codes</p>
          </div>
        </div>
        <UButton size="sm" @click="regenerate2FAModal = true">Manage</UButton>
        <UModal v-model:open="regenerate2FAModal" title="Regenerate Backup Codes" description="This will invalidate your current backup codes.">
          <template #body>
            <UForm :schema="Management2FASchema" :state="management2FAState" @submit.prevent="onManagement2FASubmit">
              <UFormField label="Password" description="Please enter your account password to proceed." :error="management2FAError" name="password">
                <UInput v-model="management2FAState.password" type="password" class="w-full" placeholder="Enter your password" autocomplete="password" />
              </UFormField>
              <UButton class="w-full flex justify-center mt-4" type="submit">Continue</UButton>
            </UForm>
          </template>
        </UModal>
      </div>

      <div class="flex items-center justify-between p-4 rounded-lg border border-dashed border-error/30">
        <div class="flex items-center gap-3">
          <UIcon name="i-heroicons:shield-exclamation" class="w-5 h-5 text-error" />
          <div>
            <p class="text-sm font-medium">Disable 2FA</p>
            <p class="text-sm text-muted">Remove two-factor authentication</p>
          </div>
        </div>
        <UButton color="error" variant="soft" size="sm" @click="disable2FAModal = true">Disable</UButton>
        <UModal v-model:open="disable2FAModal" title="Disable 2FA" description="This will reduce the security of your account.">
          <template #body>
            <UForm :schema="disable2FASchema" :state="disable2FAState" @submit.prevent="onDisable2FASubmit">
              <UFormField label="Password" description="Please enter your account password to proceed." :error="disable2FAError" name="password">
                <UInput v-model="disable2FAState.password" type="password" placeholder="Enter your password" class="w-full" autocomplete="password" />
              </UFormField>
              <UButton class="w-full flex justify-center mt-4" color="error" type="submit">Continue</UButton>
            </UForm>
          </template>
        </UModal>
      </div>
    </template>

    <!-- Reusable Backup Codes Display Modal -->
    <UModal
      v-model:open="backupCodesModal"
      title="Your Backup Codes"
      description="Save these codes in a secure location. Each code can only be used once."
      :dismissible="false"
      :close="false"
    >
      <template #body>
        <div class="grid grid-cols-2 text-sm text-center gap-3 my-6 py-8 ring ring-default rounded-lg">
          <p
            v-for="(code, index) in backupCodesDisplay"
            :key="`backup-code-${index}`"
            class="font-mono"
          >
            {{ code }}
          </p>
        </div>
        <UAlert
          color="warning"
          variant="subtle"
          icon="i-heroicons:exclamation-triangle"
          title="Important"
          description="Store these codes securely. If you lose access to your authenticator app, these codes are the only way to recover your account."
          class="mb-4 border border-warning/40 bg-warning/15 text-warning-800 dark:border-warning-400/40 dark:bg-warning-950/40 dark:text-warning-200"
        />
        <UCheckbox
          v-model="backupCodesAcknowledgement"
          label="I have saved my backup codes in a secure location"
          class="mb-4"
        />
        <UButton
          class="flex justify-center w-full"
          :disabled="!backupCodesAcknowledgement"
          @click="closeBackupCodesModal"
        >
          Done
        </UButton>
      </template>
    </UModal>
  </div>
</template>
