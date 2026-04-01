<script setup lang="ts">
import * as z from 'zod'
import type { FormSubmitEvent } from '@nuxt/ui'
import { FetchError } from 'ofetch'

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

const onSettingsSubmit = async (event: FormSubmitEvent<SettingsSchema>) => {
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
  }
}

const deleteOrgModal = ref(false)
const deleteOrgSchema = z.object({
  confirmation: z.boolean().refine(val => val === true, 'You must confirm organization deletion')
})
type DeleteOrgSchema = z.output<typeof deleteOrgSchema>
const deleteOrgState = reactive<DeleteOrgSchema>({
  confirmation: false
})

const onDeleteOrgSubmit = async () => {
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
  }
}
</script>

<template>
  <UDashboardPanel id="settings">
    <template #body>
      <UiPageContainer title="Organization Settings" size="max-w-xl">
        <div class="flex flex-col gap-4 sm:gap-6 lg:gap-12 w-full lg:max-w-2xl mx-auto">
          <div>
            <UForm
              id="org-settings"
              :schema="settingsSchema"
              :state="settings"
              @submit.prevent="onSettingsSubmit"
            >
              <UPageCard
                title="General Settings"
                description="Manage your organization details"
                variant="naked"
                orientation="horizontal"
                class="mb-4"
              >
                <UButton
                  form="org-settings"
                  label="Save changes"
                  variant="soft"
                  type="submit"
                  class="w-fit lg:ms-auto"
                />
              </UPageCard>
  
              <UPageCard class="flex flex-row gap-3">
                <UFormField
                  name="name"
                  label="Organization Name"
                  description="The name of your organization"
                  required
                  class="flex max-sm:flex-col justify-between items-start gap-4"
                >
                  <UInput
                    v-model="settings.name"
                    autocomplete="off"
                  />
                </UFormField>
              </UPageCard>
            </UForm>
          </div>
          <div>
            <UPageCard
              title="Danger Zone"
              description="Irreversible and destructive actions"
              variant="naked"
              orientation="horizontal"
              class="mb-4"
            />
            <UPageCard class="flex flex-row gap-3">
              <div class="flex items-center justify-between p-4 rounded-lg ring ring-error/30 w-full">
                <div class="flex items-center gap-3">
                  <UIcon name="i-heroicons:trash" class="w-5 h-5 text-error" />
                  <div>
                    <p class="text-sm font-medium">Delete Organization</p>
                    <p class="text-sm text-muted">Permanently delete this organization and all associated data</p>
                  </div>
                </div>
                <UButton
                  color="error"
                  variant="soft"
                  size="sm"
                  @click="deleteOrgModal = true"
                >
                  Delete
                </UButton>
                  <UModal v-model:open="deleteOrgModal" title="Delete Organization" description="This action cannot be undone.">
                    <template #body>
                      <UForm :schema="deleteOrgSchema" :state="deleteOrgState" @submit.prevent="onDeleteOrgSubmit">
                        <div class="space-y-4">
                          <UFormField
                            label="Confirmation"
                            name="confirmation"
                          >
                            <UCheckbox
                              v-model="deleteOrgState.confirmation"
                              label="I understand that this action is permanent and irreversible"
                            />
                          </UFormField>
                          <div class="flex gap-2 justify-end">
                            <UButton
                              variant="soft"
                              type="button"
                              @click="deleteOrgModal = false"
                            >
                              Cancel
                            </UButton>
                            <UButton
                              color="error"
                              type="submit"
                              :disabled="!deleteOrgState.confirmation"
                            >
                              Delete Organization
                            </UButton>
                          </div>
                        </div>
                      </UForm>
                    </template>
                  </UModal>
              </div>
            </UPageCard>
          </div>
        </div>
      </UiPageContainer>
    </template>
  </UDashboardPanel>
</template>
