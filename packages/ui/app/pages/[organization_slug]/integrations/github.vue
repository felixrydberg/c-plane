<script setup lang="ts">
import { ICONS } from '~/utils/icons'

defineOptions({ name: 'OrganizationGithubIntegrationPage' })
</script>

<template>
  <div class="mx-auto flex w-full max-w-375 flex-col gap-8">
    <div class="border-b border-default/60 pb-5">
      <UiPageEyebrow label="Integrations" />
      <h1 class="mt-2 text-2xl font-semibold">GitHub</h1>
      <p class="mt-2 max-w-2xl text-sm text-muted">
        Connect a GitHub account or Organization and deploy from selected repositories.
      </p>
    </div>

    <UCard>
      <div class="flex items-start gap-3">
        <div class="flex size-10 shrink-0 items-center justify-center rounded-lg bg-elevated">
          <UIcon :name="ICONS.link" class="size-5 text-default" />
        </div>
        <div>
          <h2 class="font-semibold">GitHub App connection</h2>
          <p class="mt-1 text-sm text-muted">Not connected</p>
        </div>
      </div>

      <div class="mt-6 rounded-lg border border-default/60 bg-elevated/40 p-4">
        <p class="text-sm font-medium">What this will enable</p>
        <ul class="mt-3 space-y-2 text-sm text-muted">
          <li>Install C-Plane on a personal account or GitHub Organization.</li>
          <li>Choose which repositories C-Plane can access.</li>
          <li>Bind a repository and branch to a container environment.</li>
          <li>Automatically deploy pushes to the configured branch.</li>
        </ul>
      </div>

      <div class="mt-6 flex items-center justify-between gap-3">
        <p class="text-xs text-dimmed">The GitHub App connection flow is not wired yet.</p>
        <UButton :icon="ICONS.link" color="primary" disabled>Connect GitHub</UButton>
      </div>
    </UCard>
  </div>
</template>
