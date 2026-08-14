path "cplane/data/platform/s3/providers/*" {
  capabilities = ["read"]
}

path "cplane/data/platform/s3/access-keys/*" {
  capabilities = ["create", "update", "read", "delete"]
}

path "cplane/metadata/platform/s3/access-keys/*" {
  capabilities = ["delete"]
}

path "cplane/data/storage/sse-c/*" {
  capabilities = ["create", "update", "read", "delete"]
}

path "cplane/metadata/storage/sse-c/*" {
  capabilities = ["delete"]
}

path "cplane/data/organizations/*" {
  capabilities = ["create", "update", "read", "delete"]
}

path "cplane/metadata/organizations/*" {
  capabilities = ["delete"]
}
