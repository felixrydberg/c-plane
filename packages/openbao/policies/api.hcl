path "transit/encrypt/platform" {
  capabilities = ["update"]
}

path "transit/decrypt/platform" {
  capabilities = ["update"]
}

path "transit/keys/tenant-*" {
  capabilities = ["create", "update", "read"]
}

path "transit/encrypt/tenant-*" {
  capabilities = ["update"]
}

path "transit/decrypt/tenant-*" {
  capabilities = ["update"]
}
