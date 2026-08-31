#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

mode="${1:-prod}"
case "$mode" in
  dev) compose_file=docker-compose.dev.yml ;;
  prod) compose_file=docker-compose.prod.yml ;;
  *) echo "Usage: $0 [dev|prod]" >&2; exit 2 ;;
esac

for command in curl docker openssl; do
  command -v "$command" >/dev/null || { echo "Missing required command: $command" >&2; exit 1; }
done
docker compose version >/dev/null 2>&1

[ -f .env ] || cp .env.example .env

env_value() {
  sed -n "s/^[[:space:]]*$1[[:space:]]*=[[:space:]]*//p" .env | tail -n 1 | sed "s/^[\"']//;s/[\"']$//"
}

set_env() {
  if grep -Eq "^[[:space:]]*$1[[:space:]]*=" .env; then
    sed -i.bak -E "s|^[[:space:]]*$1[[:space:]]*=.*|$1=$2|" .env
    rm -f .env.bak
  else
    printf '\n%s=%s\n' "$1" "$2" >> .env
  fi
}

configure_preset() {
  read -r -p "Use a public domain for ingress? [y/N]: " use_domain
  case "$use_domain" in
    y|Y|yes|YES|Yes)
      domain="$(env_value CPLANE_DOMAIN)"
      if [ -n "$domain" ]; then
        read -r -p "Public domain [$domain]: " entered_domain
        domain="${entered_domain:-$domain}"
      else
        read -r -p "Public domain (for example: example.com): " domain
      fi
      if [ -z "$domain" ]; then
        echo "A public domain is required when domain ingress is enabled" >&2
        exit 2
      fi
      if ! printf '%s\n' "$domain" | grep -Eq '^[A-Za-z0-9]([A-Za-z0-9-]{0,61}[A-Za-z0-9])?(\.[A-Za-z0-9]([A-Za-z0-9-]{0,61}[A-Za-z0-9])?)*$'; then
        echo "The domain must be a hostname without a scheme, port, path, or trailing dot" >&2
        exit 2
      fi
      set_env CPLANE_DOMAIN "$domain"
      set_env NUXT_AUTH_BASE_URL "https://$domain"
      set_env INGRESS_PLATFORM_HOSTS "$domain"
      set_env INGRESS_API_HOSTS "api.$domain"
      set_env INGRESS_STORAGE_HOSTS "storage.$domain"
      set_env INGRESS_REGISTRY_HOSTS "registry.$domain"
      set_env INGRESS_FORWARDED_PROTO https
      set_env REGISTRY_HOST "registry.$domain"
      set_env REGISTRY_INTERNAL_URL http://registry:5000
      set_env REGISTRY_TOKEN_REALM "https://api.$domain/api/registry/token"
      ;;
    *)
      if [ "$mode" = prod ]; then
        echo "Production requires a public domain for ingress" >&2
        exit 2
      fi
      set_env NUXT_AUTH_BASE_URL http://localhost:3000
      set_env INGRESS_PLATFORM_HOSTS localhost:3000
      set_env INGRESS_API_HOSTS localhost:8080
      set_env INGRESS_STORAGE_HOSTS localhost:8081
      set_env INGRESS_REGISTRY_HOSTS localhost:5000
      set_env INGRESS_FORWARDED_PROTO http
      set_env REGISTRY_HOST localhost:5000
      set_env REGISTRY_INTERNAL_URL http://registry:5000
      set_env REGISTRY_TOKEN_REALM http://localhost:8080/api/registry/token
      ;;
  esac
}

configure_preset

ensure_secret() {
  value="$(env_value "$1")"
  case "$value" in
    ""|mysecret*|your-secure-*|replace-with-*|generated-by-*) set_env "$1" "$(openssl rand -hex "${2:-32}")" ;;
  esac
}

ensure_registry_token_secret() {
  value="$(env_value REGISTRY_TOKEN_SECRET)"
  case "$value" in
    ""|replace-with-*|generated-by-*)
      set_env REGISTRY_TOKEN_SECRET "$(openssl rand 32 | openssl base64 -A | tr '+/' '-_' | tr -d '=')"
      ;;
  esac
}

new_uuid() {
  hex="$(openssl rand -hex 16)"
  printf '%s-%s-4%s-a%s-%s' "${hex:0:8}" "${hex:8:4}" "${hex:13:3}" "${hex:17:3}" "${hex:20:12}"
}

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

progress_pid=

progress_spinner() {
  local label="$1"
  local start="$2"
  local frames='|/-\\'
  local frame=0
  local elapsed

  while :; do
    elapsed=$((SECONDS - start))
    printf '\r\033[2K[%s] %s (%ss)' "${frames:frame:1}" "$label" "$elapsed" >&2
    frame=$(( (frame + 1) % 4 ))
    sleep 0.2
  done
}

cleanup_progress() {
  if [ -n "${progress_pid:-}" ]; then
    kill "$progress_pid" 2>/dev/null || true
    wait "$progress_pid" 2>/dev/null || true
    progress_pid=
    printf '\r\033[2K' >&2
  fi
}

trap cleanup_progress EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

run_with_progress() {
  local label="$1"
  local capture_output=0
  local log
  local start
  local elapsed
  local status=0

  shift
  if [ "${1:-}" = --capture-output ]; then
    capture_output=1
    shift
  fi

  log="$(mktemp)"
  start=$SECONDS
  progress_spinner "$label" "$start" &
  progress_pid=$!

  if "$@" >"$log" 2>&1; then
    status=0
  else
    status=$?
  fi

  cleanup_progress
  elapsed=$((SECONDS - start))
  if [ "$status" -eq 0 ]; then
    printf '\r\033[2K[✓] %s (%ss)\n' "$label" "$elapsed" >&2
    if [ "$capture_output" -eq 1 ]; then
      cat "$log"
    fi
  else
    printf '\r\033[2K[✗] %s (%ss)\n' "$label" "$elapsed" >&2
    cat "$log" >&2
  fi
  rm -f "$log"
  return "$status"
}

for key in POSTGRES_PASSWORD POSTGRES_UI_PASSWORD POSTGRES_IDENTITY_PASSWORD POSTGRES_TENANT_PASSWORD POSTGRES_ADMIN_PASSWORD VALKEY_PASSWORD BETTER_AUTH_SECRET CPLANE_SERVICE_TOKEN REGISTRY_HTTP_SECRET; do
  ensure_secret "$key"
done
ensure_registry_token_secret
[ -n "$(env_value REGISTRY_HOST)" ] || set_env REGISTRY_HOST localhost:5000
case "$(env_value REGISTRY_TOKEN_REALM)" in
  ""|http://localhost:3000/api/backend/registry/token) set_env REGISTRY_TOKEN_REALM http://localhost:8080/api/registry/token ;;
esac
compose=(docker compose --env-file .env -f "$compose_file")

run_with_progress "Starting Postgres, Valkey, and OpenBao" \
  "${compose[@]}" up -d --wait postgresd valkey openbao

bao_status="$("${compose[@]}" exec -T -e BAO_ADDR=http://127.0.0.1:8200 openbao bao status -format=json 2>/dev/null || true)"
if ! printf '%s' "$bao_status" | grep -Eq '"initialized"[[:space:]]*:[[:space:]]*true'; then
  if ! bao_init="$(run_with_progress "Initializing OpenBao" --capture-output \
    "${compose[@]}" exec -T -e BAO_ADDR=http://127.0.0.1:8200 openbao bao operator init -key-shares=1 -key-threshold=1)"; then
    echo "OpenBao initialization failed" >&2
    exit 1
  fi
  bao_unseal_key="$(printf '%s\n' "$bao_init" | sed -n 's/^Unseal Key 1: //p')"
  bao_root_token="$(printf '%s\n' "$bao_init" | sed -n 's/^Initial Root Token: //p')"
  if [ -z "$bao_unseal_key" ] || [ -z "$bao_root_token" ]; then
    echo "OpenBao initialization returned incomplete credentials" >&2
    exit 1
  fi
  set_env OPENBAO_UNSEAL_KEY "$bao_unseal_key"
  set_env OPENBAO_ROOT_TOKEN "$bao_root_token"
else
  bao_unseal_key="$(env_value OPENBAO_UNSEAL_KEY)"
  bao_root_token="$(env_value OPENBAO_ROOT_TOKEN)"
  if [ -z "$bao_unseal_key" ] || [ -z "$bao_root_token" ] \
    || printf '%s:%s' "$bao_unseal_key" "$bao_root_token" | grep -q generated-by-; then
    echo "OpenBao is already initialized, but .env does not contain its unseal key and root token." >&2
    exit 1
  fi
fi

run_with_progress "Unsealing OpenBao and enabling Transit" \
  "${compose[@]}" run --rm openbao-init

echo "Configuring OpenBao AppRoles..."
root_bao=("${compose[@]}" exec -T -e BAO_ADDR=http://127.0.0.1:8200 -e "BAO_TOKEN=$(env_value OPENBAO_ROOT_TOKEN)" openbao bao)
"${root_bao[@]}" write -f transit/keys/platform >/dev/null
"${root_bao[@]}" auth enable approle >/dev/null 2>&1 || true
"${root_bao[@]}" policy write cplane-api - < packages/openbao/policies/api.hcl >/dev/null
"${root_bao[@]}" policy write cplane-control-plane - < packages/openbao/policies/control-plane.hcl >/dev/null
"${root_bao[@]}" policy write cplane-worker - < packages/openbao/policies/worker.hcl >/dev/null
"${root_bao[@]}" write auth/approle/role/cplane-api token_policies=cplane-api token_ttl=1h token_max_ttl=4h >/dev/null
"${root_bao[@]}" write auth/approle/role/cplane-control-plane token_policies=cplane-control-plane token_ttl=1h token_max_ttl=4h >/dev/null
"${root_bao[@]}" write auth/approle/role/cplane-worker token_policies=cplane-worker token_ttl=1h token_max_ttl=4h >/dev/null
api_role_id="$("${root_bao[@]}" read -field=role_id auth/approle/role/cplane-api/role-id)"
api_secret_id="$("${root_bao[@]}" write -field=secret_id -f auth/approle/role/cplane-api/secret-id)"
control_plane_role_id="$("${root_bao[@]}" read -field=role_id auth/approle/role/cplane-control-plane/role-id)"
control_plane_secret_id="$("${root_bao[@]}" write -field=secret_id -f auth/approle/role/cplane-control-plane/secret-id)"
worker_role_id="$("${root_bao[@]}" read -field=role_id auth/approle/role/cplane-worker/role-id)"
worker_secret_id="$("${root_bao[@]}" write -field=secret_id -f auth/approle/role/cplane-worker/secret-id)"
set_env OPENBAO_API_ROLE_ID "$api_role_id"
set_env OPENBAO_API_SECRET_ID "$api_secret_id"
set_env OPENBAO_CONTROL_PLANE_ROLE_ID "$control_plane_role_id"
set_env OPENBAO_CONTROL_PLANE_SECRET_ID "$control_plane_secret_id"
set_env OPENBAO_WORKER_ROLE_ID "$worker_role_id"
set_env OPENBAO_WORKER_SECRET_ID "$worker_secret_id"

api_token="$("${root_bao[@]}" write -field=token auth/approle/login role_id="$api_role_id" secret_id="$api_secret_id")"
control_plane_token="$("${root_bao[@]}" write -field=token auth/approle/login role_id="$control_plane_role_id" secret_id="$control_plane_secret_id")"
api_provider_caps="$("${compose[@]}" exec -T -e BAO_ADDR=http://127.0.0.1:8200 -e "BAO_TOKEN=$api_token" openbao bao token capabilities transit/decrypt/platform)"
control_plane_tenant_caps="$("${compose[@]}" exec -T -e BAO_ADDR=http://127.0.0.1:8200 -e "BAO_TOKEN=$control_plane_token" openbao bao token capabilities transit/encrypt/tenant-policy-smoke-test)"
printf '%s' "$api_provider_caps" | grep -qw update
[ "$control_plane_tenant_caps" = deny ] || { echo "Control-plane AppRole unexpectedly has tenant access" >&2; exit 1; }
unset api_token control_plane_token api_secret_id control_plane_secret_id worker_role_id worker_secret_id

run_with_progress "Applying database migrations" \
  "${compose[@]}" run --rm --build migrate

provision_tenant_keys() {
  local organization_id

  while IFS= read -r organization_id; do
    if [ -n "$organization_id" ]; then
      "${root_bao[@]}" write -f "transit/keys/tenant-${organization_id//-/}" >/dev/null || return 1
    fi
  done < <("${compose[@]}" exec -T postgresd psql -U cplane -d cplane -At -c 'SELECT id FROM organization')
}

run_with_progress "Provisioning tenant Transit keys" provision_tenant_keys

provider_id="$("${compose[@]}" exec -T postgresd psql -U cplane -d cplane -At \
  -c "SELECT id FROM s3_providers WHERE is_active=true ORDER BY created_at LIMIT 1")"
provider_exists=0
[ -z "$provider_id" ] || provider_exists=1

if [ "$provider_exists" != 1 ]; then
  echo
  echo "Configure the first S3 provider"
  while :; do
    read -r -p "S3 provider name: " provider_name
    if [ -z "$provider_name" ]; then
      echo "S3 provider name is required; try again." >&2
      continue
    fi
    default_provider_region=us-east-1

    read -r -p "S3 endpoint URL: " provider_endpoint
    if [ -z "$provider_endpoint" ]; then
      echo "S3 endpoint URL is required; try again." >&2
      continue
    fi
    read -r -p "S3 signing region ($default_provider_region): " provider_region
    provider_region="${provider_region:-$default_provider_region}"
    read -r -p "Access key ID: " provider_access_key
    if [ -z "$provider_access_key" ]; then
      echo "Access key ID is required; try again." >&2
      continue
    fi
    read -r -s -p "Secret access key: " provider_secret_key
    echo
    if [ -z "$provider_secret_key" ]; then
      echo "Secret access key is required; try again." >&2
      continue
    fi
    read -r -s -p "Session token (input hidden; press Enter to skip): " provider_session_token
    echo

    read -r -p "Default C-Plane region slug ($provider_region): " region_slug
    region_slug="${region_slug:-$provider_region}"
    if ! printf '%s' "$region_slug" | grep -Eq '^[a-z0-9]+(-[a-z0-9]+)*$'; then
      echo "Region slug must contain lowercase letters, numbers, and single hyphens; try again." >&2
      continue
    fi
    read -r -p "Default C-Plane region name ($region_slug): " region_name
    region_name="${region_name:-$region_slug}"
    break
  done
  provider_id="$(new_uuid)"
  provider_secret_id="$(new_uuid)"
  region_id="$(new_uuid)"
  if [ -n "$provider_session_token" ]; then
    session_token_json="\"$(json_escape "$provider_session_token")\""
  else
    session_token_json="null"
  fi
  provider_payload="{\"access_key_id\":\"$(json_escape "$provider_access_key")\",\"secret_access_key\":\"$(json_escape "$provider_secret_key")\",\"session_token\":${session_token_json}}"
  provider_payload_b64="$(printf '%s' "$provider_payload" | openssl base64 -A)"
  provider_ciphertext="$(printf '%s' "$provider_payload_b64" | "${root_bao[@]}" write -field=ciphertext transit/encrypt/platform plaintext=-)"
  unset provider_payload provider_payload_b64 session_token_json

  if ! run_with_progress "Saving S3 provider configuration" "${compose[@]}" exec -T postgresd psql -v ON_ERROR_STOP=1 -U cplane -d cplane \
    -v provider_id="$provider_id" -v provider_name="$provider_name" \
    -v provider_endpoint="$provider_endpoint" -v provider_region="$provider_region" \
    -v provider_secret_id="$provider_secret_id" -v provider_ciphertext="$provider_ciphertext" \
    -v region_id="$region_id" -v region_slug="$region_slug" -v region_name="$region_name" <<'SQL'
BEGIN;
INSERT INTO secret (id, scope, ciphertext)
VALUES (:'provider_secret_id'::uuid, 'platform', :'provider_ciphertext');
INSERT INTO s3_providers (id, name, endpoint_url, provider_region, credential_secret_id, is_active)
VALUES (:'provider_id'::uuid, :'provider_name', :'provider_endpoint', :'provider_region', :'provider_secret_id'::uuid, true);
INSERT INTO regions (id, slug, display_name, s3_provider_id, status, routing_mode)
VALUES (:'region_id'::uuid, :'region_slug', :'region_name', :'provider_id'::uuid, 'active', 'active');
INSERT INTO infrastructure_audit_log (id, actor_identifier, source_ip, action, resource_type, resource_id, changes)
VALUES
  (gen_random_uuid(), 'install.sh', 'local', 'create', 's3_provider', :'provider_id'::uuid, jsonb_build_object('name', :'provider_name', 'endpoint_url', :'provider_endpoint', 'provider_region', :'provider_region', 'is_active', true)),
  (gen_random_uuid(), 'install.sh', 'local', 'create', 'region', :'region_id'::uuid, jsonb_build_object('slug', :'region_slug', 'display_name', :'region_name', 'status', 'active', 's3_provider_id', :'provider_id'));
COMMIT;
SQL
  then
    exit 1
  fi
  unset provider_secret_key provider_session_token
fi

services=(ingress storage ui api registry control-plane worker)
run_with_progress "Starting C-Plane ($mode)" \
  "${compose[@]}" up -d --build "${services[@]}"
echo "C-Plane is installed"
