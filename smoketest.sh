#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

registry_host="${REGISTRY_HOST:-localhost:5000}"
registry_url="${CPLANE_REGISTRY_URL:-http://$registry_host}"
registry_url="${registry_url%/}"
token_url="${CPLANE_REGISTRY_TOKEN_URL:-http://localhost:8080/api/registry/token}"
status="$(curl -ksS -o /dev/null -w '%{http_code}' "$registry_url/v2/" || true)"
if [ "$status" != 401 ]; then
  echo "Registry failed: $registry_url/v2/ returned ${status:-no response}" >&2
  exit 1
fi
printf '%-14s OK (%s)\n' Registry "$status"

registry_repository="${REGISTRY_REPOSITORY:-project-x}"
registry_username="${REGISTRY_USERNAME:-}"
registry_password="${REGISTRY_PASSWORD:-}"
if [ -t 0 ]; then
  read -r -p "Registry username${registry_username:+ [$registry_username]}: " input
  registry_username="${input:-$registry_username}"
  read -r -s -p 'Registry password: ' input
  printf '\n'
  registry_password="${input:-$registry_password}"
fi

if [ -z "$registry_username" ] || [ -z "$registry_password" ]; then
  echo 'Registry username and password are required.' >&2
  exit 1
fi

image_tag="${SMOKE_TAG:-$registry_repository}"
image="$registry_host/$registry_username/$registry_repository:$image_tag"
printf '%s' "$registry_password" | docker login "$registry_host" --username "$registry_username" --password-stdin
docker pull "${SMOKE_IMAGE:-alpine:3.20}"
docker tag "${SMOKE_IMAGE:-alpine:3.20}" "$image"
docker push "$image"
token_response="$(curl -fsS --user "$registry_username:$registry_password" --get "$token_url" \
  --data-urlencode "service=$registry_host" \
  --data-urlencode "scope=repository:$registry_username/$registry_repository:pull,push,delete")"
token="$(printf '%s' "$token_response" | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')"
[ -n "$token" ] || { echo 'Registry token request returned no token.' >&2; exit 1; }
manifest_digest="$(curl -ksSI -H "Authorization: Bearer $token" "$registry_url/v2/$registry_username/$registry_repository/manifests/$image_tag" \
  | tr -d '\r' | sed -n 's/^Docker-Content-Digest: //Ip' | head -n 1)"
[ -n "$manifest_digest" ] || { echo 'Registry manifest did not return a digest.' >&2; exit 1; }
curl -kfsS -o /dev/null -X DELETE -H "Authorization: Bearer $token" \
  "$registry_url/v2/$registry_username/$registry_repository/manifests/$manifest_digest"
docker image rm "$image" >/dev/null
printf '%-14s OK (%s)\n' Remote-delete "$registry_username/$registry_repository@$manifest_digest"
printf '%-14s OK (%s)\n' Push/remove "$image"
