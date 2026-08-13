path "cplane/data/platform/s3/providers/*" {
  capabilities = ["create", "update", "read", "delete"]
}

path "cplane/metadata/platform/s3/providers/*" {
  capabilities = ["delete"]
}
