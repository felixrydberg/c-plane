<script setup lang="ts">
import * as z from 'zod'
import type { FormSubmitEvent } from '@nuxt/ui'
import useStore from '~/stores/store'
import { createClient } from '~/utils/auth'
import { changePasswordSchema } from '~/utils/validation'

const store = useStore();
const toast = useToast();
const client = createClient();
if (!store.user) {
  throw createError('User not found in store')
}

const profileSchema = z.object({
  name: z.string().min(2, 'Too short'),
  email: z.email('Invalid email'),
})

type ProfileSchema = z.output<typeof profileSchema>
const profile = reactive<Partial<ProfileSchema>>({
  name: store.user.name,
  email: store.user.email,
})

const isProfileLoading = ref(false);

const onProfileSubmit = async (event: FormSubmitEvent<ProfileSchema>) => {
  try {
    isProfileLoading.value = true;
    
    if (event.data.name !== store.user?.name) {
      await $fetch('/api/user/profile', {
        method: 'PATCH',
        body: {
          name: event.data.name
        }
      });
      store.user!.name = event.data.name;
    }
    
    if (event.data.email !== store.user?.email) {
      const { error } = await client.changeEmail({
        newEmail: event.data.email
      });
      
      if (error) {
        toast.add({
          title: 'Error',
          description: error.message || 'Failed to change email',
          icon: 'i-lucide-x',
          color: 'error'
        });
        return;
      }
      
      store.user!.email = event.data.email;
    }
    
    toast.add({
      title: 'Success',
      description: 'Your settings have been updated.',
      icon: 'i-lucide-check',
      color: 'success'
    });
  } catch (error) {
    toast.add({
      title: 'Error',
      description: error instanceof Error ? error.message : 'Failed to update profile',
      icon: 'i-lucide-x',
      color: 'error'
    });
  } finally {
    isProfileLoading.value = false;
  }
}

const isPasswordLoading = ref(false)
const passwordSchema = changePasswordSchema

type PasswordSchema = z.output<typeof passwordSchema>
const passwordError = ref<string>()
const passwordState = reactive<PasswordSchema>({
  currentPassword: '',
  newPassword: '',
  confirmPassword: '',
})

const onChangePassword = async () => {
  try {
    passwordError.value = undefined
    isPasswordLoading.value = true

    const { error } = await client.changePassword({
      currentPassword: passwordState.currentPassword,
      newPassword: passwordState.newPassword,
    })

    if (error) {
      if (error.code === 'INVALID_PASSWORD') {
        passwordError.value = 'The current password you entered is incorrect.'
      } else {
        passwordError.value = error.message || 'Failed to change password'
      }
      return
    }

    toast.add({
      title: 'Success',
      description: 'Your password has been changed.',
      icon: 'i-lucide-check',
      color: 'success'
    })

    passwordState.currentPassword = ''
    passwordState.newPassword = ''
    passwordState.confirmPassword = ''
  } catch (error) {
    passwordError.value = error instanceof Error ? error.message : 'Failed to change password'
  } finally {
    isPasswordLoading.value = false
  }
}

const deleteAccountModal = ref(false)
const deleteAccountSchema = z.object({
  password: z.string().min(8, 'Password is required'),
  confirmation: z.boolean().refine(val => val === true, 'You must confirm account deletion')
})
type DeleteAccountSchema = z.output<typeof deleteAccountSchema>
const deleteAccountError = ref<string>()
const deleteAccountState = reactive<DeleteAccountSchema>({
  password: '',
  confirmation: false
})

const onDeleteAccountSubmit = async () => {
  const { error } = await client.deleteUser({
    password: deleteAccountState.password,
  })

  if (error) {
    if (error.code === 'INVALID_PASSWORD') {
      deleteAccountError.value = 'The password you entered is incorrect.'
    } else {
      deleteAccountError.value = undefined
      toast.add({
        title: 'Error deleting account',
        description: error.message,
        color: 'error'
      })
    }
    return
  }

  toast.add({
    title: 'Account Deleted',
    description: 'Your account has been permanently deleted.',
    color: 'success'
  })
  
  deleteAccountModal.value = false
  deleteAccountState.password = ''
  deleteAccountState.confirmation = false
  deleteAccountError.value = undefined

  store.$reset();
  navigateTo('/')
}
</script>

<template>

  <UForm
    id="settings"
    :schema="profileSchema"
    :state="profile"
    @submit.prevent="onProfileSubmit"
  >
    <div class="w-full border border-default rounded-lg p-6 space-y-6">
      <div class="flex items-center justify-between">
        <div>
          <p class="font-semibold">Profile</p>
          <p class="text-sm text-muted">These informations will be displayed publicly.</p>
        </div>
        <UButton
          form="settings"
          label="Save changes"
          type="submit"
          :loading="isProfileLoading"
          :disabled="isProfileLoading"
          size="sm"
        />
      </div>

      <div class="flex flex-row gap-6">
        <div class="flex-1">
          <UFormField name="name" label="Name" required>
            <UInput v-model="profile.name" autocomplete="off" class="w-full" />
          </UFormField>
        </div>
        <USeparator orientation="vertical" class="h-auto" />
        <div class="flex-1">
          <UFormField name="email" label="Email" required>
            <UInput v-model="profile.email" type="email" autocomplete="off" class="w-full" />
          </UFormField>
        </div>
      </div>
    </div>
  </UForm>

  <UForm :schema="passwordSchema" :state="passwordState" @submit.prevent="onChangePassword">
    <div class="w-full border border-default rounded-lg p-6 space-y-6">
      <div class="flex items-center justify-between">
        <div>
          <p class="font-semibold">Password</p>
          <p class="text-sm text-muted">Change your account password</p>
        </div>
        <UButton
          label="Change Password"
          type="submit"
          :loading="isPasswordLoading"
          :disabled="isPasswordLoading"
          size="sm"
        />
      </div>

      <div class="flex flex-col gap-4">
        <UFormField
          name="currentPassword"
          label="Current Password"
          :error="passwordError"
          required
        >
          <UInput
            v-model="passwordState.currentPassword"
            type="password"
            autocomplete="current-password"
            placeholder="Enter your current password"
            class="w-full"
          />
        </UFormField>
        <USeparator />
        <UFormField name="newPassword" label="New Password" required>
          <UInput
            v-model="passwordState.newPassword"
            type="password"
            autocomplete="new-password"
            placeholder="Enter your new password"
            class="w-full"
          />
        </UFormField>
        <USeparator />
        <UFormField name="confirmPassword" label="Confirm Password" required>
          <UInput
            v-model="passwordState.confirmPassword"
            type="password"
            autocomplete="new-password"
            placeholder="Confirm your new password"
            class="w-full"
          />
        </UFormField>
      </div>
    </div>
  </UForm>

  <dashboard-settings-profile-2FA />

  <div class="w-full border border-default rounded-lg p-6 space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <p class="font-semibold">Danger Zone</p>
        <p class="text-sm text-muted">Irreversible and destructive actions</p>
      </div>
    </div>

    <div class="flex items-center justify-between p-4 rounded-lg border border-dashed border-error/30">
      <div class="flex items-center gap-3">
        <UIcon name="i-heroicons:trash" class="w-5 h-5 text-error" />
        <div>
          <p class="text-sm font-medium">Delete Account</p>
          <p class="text-sm text-muted">Permanently delete your account and all associated data</p>
        </div>
      </div>
      <UButton
        color="error"
        variant="soft"
        size="sm"
        @click="deleteAccountModal = true"
      >
        Delete
      </UButton>
      <UModal v-model:open="deleteAccountModal" title="Delete Account" description="This action cannot be undone. Please enter your password to confirm.">
        <template #body>
          <UForm :schema="deleteAccountSchema" :state="deleteAccountState" @submit.prevent="onDeleteAccountSubmit">
            <div class="space-y-4">
              <UFormField
                label="Password"
                description="Please enter your account password to proceed."
                :error="deleteAccountError"
                name="password"
              >
                <UInput
                  v-model="deleteAccountState.password"
                  type="password"
                  class="w-full"
                  placeholder="Enter your password"
                  autocomplete="password"
                />
              </UFormField>
              <UFormField label="Confirmation" name="confirmation">
                <UCheckbox
                  v-model="deleteAccountState.confirmation"
                  label="I understand that this action is permanent and irreversible"
                />
              </UFormField>
              <div class="flex justify-end gap-3 pt-2">
                <UButton variant="ghost" color="neutral" type="button" @click="deleteAccountModal = false">Cancel</UButton>
                <UButton color="error" type="submit" :disabled="!deleteAccountState.confirmation">Delete My Account</UButton>
              </div>
            </div>
          </UForm>
        </template>
      </UModal>
    </div>
  </div>
</template>
