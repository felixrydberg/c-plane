import type { Environment } from '@cplane/sdk'

type EnvironmentStore = {
  environment: Environment | null
  environments: Environment[]
}

export function syncEnvironment(store: EnvironmentStore, updated: Environment) {
  if (store.environment?.id === updated.id) store.environment = updated
  const index = store.environments.findIndex(environment => environment.id === updated.id)
  if (index !== -1) store.environments[index] = updated
}
