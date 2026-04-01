<script setup lang="ts">
import z from 'zod';
import { useDebounceFn } from '@vueuse/core';

const store = useStore();
const toast = useToast();
const router = useRouter();
const goBack = async () => {
  if (store.organization?.slug) {
    await router.push(`/${store.organization.slug}`);
  }
};
if (!store.user) {
  throw createError("User not found in store");
}

const schema = z.object({
  name: z.string(),
  email: z.email("Invalid email address"),
  slug: z.string()
    .regex(ORGANIZATION_SLUG_REGEX, "Slug can only contain lowercase letters, numbers, and hyphens, and must start and end with a letter or number"),
})

type Schema = z.infer<typeof schema>;
const state = reactive<Schema>({
  name: "",
  email: "",
  slug: "",
});

const slugAvailable = ref<boolean | null>(null);
const slugValidating = ref(false);
const slugError = ref<string | null>(null);

const emailAvailable = ref<boolean | null>(null);
const emailValidating = ref(false);
const emailError = ref<string | null>(null);
const agreedToTerms = ref(false);
const manageInvitesOpen = ref(false);

const { data: pendingInvitations, refresh: refreshPendingInvitations } = await useFetch('/api/user/invitations', {
  query: {
    status: 'pending',
    limit: 1,
    offset: 0,
  },
});

const pendingInvitationCount = computed(() => pendingInvitations.value?.pagination.total || 0);
const hasPendingInvitations = computed(() => pendingInvitationCount.value > 0);

const isCheckingOut = ref(false);
const isCreateDisabled = computed(() => {
  const hasRequiredFields = state.name.trim().length > 0
    && state.slug.trim().length > 0
    && state.email.trim().length > 0;

  return !hasRequiredFields
    || isCheckingOut.value
    || slugValidating.value
    || emailValidating.value
    || slugAvailable.value === false
    || emailAvailable.value === false
    || !agreedToTerms.value;
});

const validateSlug = async (slug: string) => {
  if (!slug || slug.trim().length === 0) {
    slugAvailable.value = null;
    slugError.value = null;
    return;
  }

  slugValidating.value = true;
  try {
    const data = await $fetch("/api/organization/validate-slug", {
      method: "POST",
      body: { slug },
    });
    
    if (data) {
      slugAvailable.value = !data.exists;
      slugError.value = data.exists ? "Slug is already taken" : null;
    }
  } catch (e) {
    console.error("Error validating slug:", e);
    slugError.value = "Error checking slug availability";
  } finally {
    slugValidating.value = false;
  }
};

const validateEmail = async (email: string) => {
  if (!email || email.trim().length === 0) {
    emailAvailable.value = null;
    emailError.value = null;
    return;
  }

  emailValidating.value = true;
  try {
    const data = await $fetch("/api/organization/validate-email", {
      method: "POST",
      body: { email },
    });
    
    if (data) {
      emailAvailable.value = !data.exists;
      emailError.value = data.exists ? "Email already claimed" : null;
    }
  } catch (e) {
    console.error("Error validating email:", e);
    emailError.value = "Error checking email availability";
  } finally {
    emailValidating.value = false;
  }
};

const debouncedValidateSlug = useDebounceFn(validateSlug, 500);
const debouncedValidateEmail = useDebounceFn(validateEmail, 500);

const generateSlugFromName = (name: string): string => {
  return name
    .toLowerCase()
    .trim()
    .replace(/\s+/g, '-')
    .replace(/[^a-z0-9-]/g, '')
    .replace(/-+/g, '-')
    .replace(/^-+|-+$/g, '');
};

watch(() => state.slug, (newSlug) => {
  debouncedValidateSlug(newSlug);
});

watch(() => state.email, (newEmail) => {
  debouncedValidateEmail(newEmail);
});

watch(() => state.name, (newName) => {
  if (newName) {
    state.slug = generateSlugFromName(newName);
  }
});

const onCreateOrganization = async () => {
  if (!store.session) {
    throw createError("Session not found in store");
  }
  
  isCheckingOut.value = true;
  try {
    const data = await $fetch<Organization>("/api/organization", {
      method: "POST",
      body: {
        name: state.name,
        email: state.email,
        slug: state.slug,
      },
    });

    if (!data || !data.slug) {
      toast.add({
        color: "error",
        title: "Error creating organization",
        description: "Please try again later.",
      });
      return;
    }

    store.organization = data;
    await router.push(`/${data.slug}`);
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  } catch (error) {
    toast.add({
      color: "error",
      title: "Error creating organization",
      description: "Please try again later.",
    });
  } finally {
    isCheckingOut.value = false;
  }
};

const openManageInvites = () => {
  manageInvitesOpen.value = true;
};

const onInvitationUpdated = async () => {
  await refreshPendingInvitations();
};
</script>

<template>
  <div class="h-dvh w-full bg-default/30 px-4 py-6 md:px-6 flex items-center justify-center">
    <div class="w-full max-w-xl rounded-3xl border border-default/70 bg-default p-7 md:p-10 shadow-sm">
      <div class="mb-8 text-center space-y-3">
        <div class="mx-auto flex h-10 w-10 items-center justify-center rounded-full border border-default">
          <UIcon name="i-heroicons-building-office-2" class="size-5" />
        </div>
        <h1 class="text-2xl font-semibold text-highlighted">Let's get started</h1>
        <p class="text-sm text-muted">Create your organization to continue setting up your workspace.</p>
      </div>

      <UForm :state="state" :schema="schema" class="space-y-6" @submit.prevent="onCreateOrganization">
        <UFormField
          name="name"
          label="Organization Name"
          class="space-y-1"
        >
          <UInput
            v-model="state.name"
            type="text"
            :placeholder="`${store.user?.name}'s Organization`"
            class="w-full"
          />
          <p class="pt-1 text-xs text-muted">
            This is the display name your team and customers will see across the app.
          </p>
        </UFormField>

        <UFormField
          name="slug"
          label="Organization Slug"
          :error="slugError ? slugError : undefined"
          class="space-y-1"
        >
          <UInput
            v-model="state.slug"
            type="text"
            :placeholder="`${store.user?.name}s-organization`"
            :loading="slugValidating"
            class="w-full"
          />
          <p class="pt-1 text-xs text-muted">
            Used in your workspace URL. Keep it short, readable, and unique.
          </p>
        </UFormField>

        <UFormField
          name="email"
          label="Organization Email"
          :error="emailError ? emailError : undefined"
          class="space-y-1"
        >
          <UInput
            v-model="state.email"
            type="email"
            placeholder="organization@example.com"
            :loading="emailValidating"
            class="w-full"
          />
          <p class="pt-1 text-xs text-muted">
            We use this to notify organization admins about important events, alerts, billing updates, and account activity.
          </p>
        </UFormField>

        <div class="rounded-xl border border-default/70 bg-default/50 px-4 py-4">
          <UCheckbox v-model="agreedToTerms" label="I confirm and agree to the terms below" />
          <ul class="mt-4 list-disc space-y-2 pl-5 text-xs text-muted">
            <li>Acceptable use policy</li>
            <li>Account review and compliance checks</li>
            <li>Terms of service and privacy policy</li>
          </ul>
        </div>

        <UButton type="submit" size="lg" class="w-full flex justify-center mt-1" :loading="isCheckingOut" :disabled="isCreateDisabled">
          Create
        </UButton>

        <UButton
          v-if="hasPendingInvitations"
          type="button"
          variant="soft"
          size="lg"
          class="w-full flex justify-center"
          @click="openManageInvites"
        >
          Manage pending invites ({{ pendingInvitationCount }})
        </UButton>

        <UButton v-if="store.organizations.length > 0" variant="link" size="lg" class="text-muted underline w-full flex justify-center pt-1" @click="goBack">
          Back to Dashboard
        </UButton>
        <UButton v-else variant="link" size="lg" class="text-muted underline w-full flex justify-center pt-1" @click="signOut">
          Sign Out
        </UButton>
      </UForm>
    </div>

    <UModal
      v-model:open="manageInvitesOpen"
      title="Manage Invitations"
      description="Review your pending organization invitations."
    >
      <template #body>
        <dashboard-invitations @accepted="onInvitationUpdated" @declined="onInvitationUpdated" />
      </template>
    </UModal>
  </div>
</template>
