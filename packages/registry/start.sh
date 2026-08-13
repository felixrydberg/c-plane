#!/bin/sh
set -eu
umask 077

: "${REGISTRY_TOKEN_SECRET:?REGISTRY_TOKEN_SECRET is required}"
printf '{"keys":[{"kty":"oct","use":"sig","alg":"HS256","kid":"cplane-registry","k":"%s"}]}' \
  "$REGISTRY_TOKEN_SECRET" > /run/registry-token-jwks.json

exec registry serve /etc/distribution/config.yml
