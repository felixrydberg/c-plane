<script setup lang="ts">
import * as z from 'zod'
import type { FormSubmitEvent } from '@nuxt/ui'
import { FetchError } from 'ofetch'
import { ICONS } from '~/utils/icons'

const store = useStore();
const toast = useToast();
const router = useRouter();

if (!store.organization?.id) {
  throw createError('Organization not found in store')
}

const settingsSchema = z.object({
  name: z.string().min(2, 'Name must be at least 2 characters'),
})

type SettingsSchema = z.output<typeof settingsSchema>
const settings = reactive<Partial<SettingsSchema>>({
  name: store.organization.name,
})
const isSaving = ref(false)

const onSettingsSubmit = async (event: FormSubmitEvent<SettingsSchema>) => {
  isSaving.value = true
  try {
    const updated = await $fetch(`/api/organization/${store.organization?.id as ':organization_id'}/name`, {
      method: 'PUT',
      body: {
        name: event.data.name,
      }
    })

    if (!updated) {
      throw new Error('Failed to update organization')
    }

    store.organization = updated
    toast.add({
      title: 'Success',
      description: 'Organization name updated.',
      color: 'success'
    })
  } catch (error) {
    if (error instanceof FetchError) {
      toast.add({
        title: 'Error updating organization',
        description: error.data?.statusMessage || 'An unknown error occurred',
        color: 'error'
      })
    } else {
      toast.add({
        title: 'Error updating organization',
        description: error instanceof Error ? error.message : 'An unknown error occurred',
        color: 'error'
      })
    }
  } finally {
    isSaving.value = false
  }
}

const deleteOrgModal = ref(false)
const isDeleting = ref(false)
const deleteOrgSchema = z.object({
  confirmation: z.boolean().refine(val => val === true, 'You must confirm organization deletion')
})
type DeleteOrgSchema = z.output<typeof deleteOrgSchema>
const deleteOrgState = reactive<DeleteOrgSchema>({
  confirmation: false
})

const onDeleteOrgSubmit = async () => {
  isDeleting.value = true
  try {
    if (!store.organization?.id) {
      throw new Error('No organization selected')
    }

    await $fetch(`/api/organization/${store.organization.id as ':organization_id'}`, {
      method: 'DELETE'
    })

    toast.add({
      title: 'Organization Deleted',
      description: 'Your organization has been permanently deleted.',
      color: 'success'
    })
    
    deleteOrgModal.value = false
    deleteOrgState.confirmation = false

    store.$reset()
    await router.push('/')
  } catch (error) {
    if (error instanceof FetchError) {
      toast.add({
        title: 'Error deleting organization',
        description: error.data?.statusMessage || 'An unknown error occurred',
        color: 'error'
      })
    } else {
      toast.add({
        title: 'Error deleting organization',
        description: error instanceof Error ? error.message : 'An unknown error occurred',
        color: 'error'
      })
    }
  } finally {
    isDeleting.value = false
  }
}
</script>

<template>
  <OrganizationSettingsPage title="General">

    <UForm
      id="org-settings"
      :schema="settingsSchema"
      :state="settings"
      class="grid gap-8 border-b border-dashed border-default pb-10 lg:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)]"
      @submit.prevent="onSettingsSubmit"
    >
      <div>
        <h2 class="text-xl font-normal tracking-[-0.02em]">Organization details</h2>
        <p class="mt-2 max-w-sm text-sm text-muted">Set the name shown throughout C-Plane.</p>
      </div>
      <div>
        <UFormField name="name" label="Organization name" required>
          <UInput v-model="settings.name" autocomplete="off" class="w-full" />
        </UFormField>
        <div class="mt-6 flex justify-end">
          <UButton form="org-settings" :icon="ICONS.check" color="primary" type="submit" :loading="isSaving">
            Save changes
          </UButton>
        </div>
      </div>
    </UForm>

    <section class="grid gap-8 border-b border-dashed border-error/50 pb-10 lg:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)]">
      <div>
        <h2 class="text-xl font-normal tracking-[-0.02em] text-error">Danger zone</h2>
        <p class="mt-2 max-w-sm text-sm text-muted">Irreversible actions for this organization.</p>
      </div>
      <div>
        <div>
          <p class="text-sm font-medium">Delete organization</p>
          <p class="mt-1 text-sm text-muted">Permanently delete this organization and its data.</p>
        </div>
        <UButton class="mt-5" :icon="ICONS.trash" color="error" @click="deleteOrgModal = true">Delete organization</UButton>
      </div>
    </section>

    <UModal v-model:open="deleteOrgModal" title="Delete Organization" description="This action cannot be undone.">
      <template #body>
        <UForm :schema="deleteOrgSchema" :state="deleteOrgState" @submit.prevent="onDeleteOrgSubmit">
          <div class="space-y-4">
            <UFormField label="Confirmation" name="confirmation">
              <UCheckbox
                v-model="deleteOrgState.confirmation"
                label="I understand that this action is permanent and irreversible"
              />
            </UFormField>
            <div class="flex justify-end gap-3 pt-2">
              <UButton variant="ghost" color="neutral" type="button" @click="deleteOrgModal = false">Cancel</UButton>
              <UButton :icon="ICONS.trash" color="error" type="submit" :disabled="!deleteOrgState.confirmation" :loading="isDeleting">Delete Organization</UButton>
            </div>
          </div>
        </UForm>
      </template>
    </UModal>
  </OrganizationSettingsPage>
</template>
