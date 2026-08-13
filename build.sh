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

run_quiet() {
  local log
  log="$(mktemp)"
  if "$@" >"$log" 2>&1; then
    rm -f "$log"
  else
    cat "$log" >&2
    rm -f "$log"
    return 1
  fi
}

for key in POSTGRES_PASSWORD POSTGRES_UI_PASSWORD POSTGRES_IDENTITY_PASSWORD POSTGRES_TENANT_PASSWORD POSTGRES_ADMIN_PASSWORD VALKEY_PASSWORD BETTER_AUTH_SECRET CPLANE_SERVICE_TOKEN REGISTRY_HTTP_SECRET REGISTRY_STORAGE_S3_SECRETKEY REGISTRY_STORAGE_S3_GC_SECRETKEY; do
  ensure_secret "$key"
done
ensure_registry_token_secret
ensure_secret REGISTRY_STORAGE_S3_ACCESSKEY 16
ensure_secret REGISTRY_STORAGE_S3_GC_ACCESSKEY 16
[ -n "$(env_value STORAGE_ENDPOINT_URL)" ] || set_env STORAGE_ENDPOINT_URL http://localhost:8081
[ -n "$(env_value REGISTRY_HOST)" ] || set_env REGISTRY_HOST localhost:5000
[ -n "$(env_value REGISTRY_STORAGE_S3_REGION)" ] || set_env REGISTRY_STORAGE_S3_REGION us-east-1
[ -n "$(env_value REGISTRY_STORAGE_S3_BUCKET)" ] || set_env REGISTRY_STORAGE_S3_BUCKET cplane-registry
[ -n "$(env_value REGISTRY_STORAGE_S3_PHYSICAL_BUCKET)" ] || set_env REGISTRY_STORAGE_S3_PHYSICAL_BUCKET "$(env_value REGISTRY_STORAGE_S3_BUCKET)"
case "$(env_value REGISTRY_TOKEN_REALM)" in
  ""|http://localhost:3000/api/backend/registry/token) set_env REGISTRY_TOKEN_REALM http://localhost:8080/api/registry/token ;;
esac

compose=(docker compose --env-file .env -f "$compose_file")

echo "Starting Postgres, Valkey, and OpenBao..."
run_quiet "${compose[@]}" up -d --wait postgresd valkey openbao

bao_status="$("${compose[@]}" exec -T -e BAO_ADDR=http://127.0.0.1:8200 openbao bao status -format=json 2>/dev/null || true)"
if ! printf '%s' "$bao_status" | grep -Eq '"initialized"[[:space:]]*:[[:space:]]*true'; then
  echo "Initializing OpenBao..."
  bao_init="$("${compose[@]}" exec -T -e BAO_ADDR=http://127.0.0.1:8200 openbao bao operator init -key-shares=1 -key-threshold=1)"
  set_env OPENBAO_UNSEAL_KEY "$(printf '%s\n' "$bao_init" | sed -n 's/^Unseal Key 1: //p')"
  set_env OPENBAO_ROOT_TOKEN "$(printf '%s\n' "$bao_init" | sed -n 's/^Initial Root Token: //p')"
else
  bao_unseal_key="$(env_value OPENBAO_UNSEAL_KEY)"
  bao_root_token="$(env_value OPENBAO_ROOT_TOKEN)"
  if [ -z "$bao_unseal_key" ] || [ -z "$bao_root_token" ] \
    || printf '%s:%s' "$bao_unseal_key" "$bao_root_token" | grep -q generated-by-; then
    echo "OpenBao is already initialized, but .env does not contain its unseal key and root token." >&2
    exit 1
  fi
fi

echo "Unsealing OpenBao and enabling its KV store..."
run_quiet "${compose[@]}" run --rm openbao-init

echo "Configuring OpenBao AppRoles..."
root_bao=("${compose[@]}" exec -T -e BAO_ADDR=http://127.0.0.1:8200 -e "BAO_TOKEN=$(env_value OPENBAO_ROOT_TOKEN)" openbao bao)
"${root_bao[@]}" auth enable approle >/dev/null 2>&1 || true
"${root_bao[@]}" policy write cplane-api - < packages/openbao/policies/api.hcl >/dev/null
"${root_bao[@]}" policy write cplane-control-plane - < packages/openbao/policies/control-plane.hcl >/dev/null
"${root_bao[@]}" write auth/approle/role/cplane-api token_policies=cplane-api token_ttl=1h token_max_ttl=4h >/dev/null
"${root_bao[@]}" write auth/approle/role/cplane-control-plane token_policies=cplane-control-plane token_ttl=1h token_max_ttl=4h >/dev/null
api_role_id="$("${root_bao[@]}" read -field=role_id auth/approle/role/cplane-api/role-id)"
api_secret_id="$("${root_bao[@]}" write -field=secret_id -f auth/approle/role/cplane-api/secret-id)"
control_plane_role_id="$("${root_bao[@]}" read -field=role_id auth/approle/role/cplane-control-plane/role-id)"
control_plane_secret_id="$("${root_bao[@]}" write -field=secret_id -f auth/approle/role/cplane-control-plane/secret-id)"
set_env OPENBAO_API_ROLE_ID "$api_role_id"
set_env OPENBAO_API_SECRET_ID "$api_secret_id"
set_env OPENBAO_CONTROL_PLANE_ROLE_ID "$control_plane_role_id"
set_env OPENBAO_CONTROL_PLANE_SECRET_ID "$control_plane_secret_id"

api_token="$("${root_bao[@]}" write -field=token auth/approle/login role_id="$api_role_id" secret_id="$api_secret_id")"
control_plane_token="$("${root_bao[@]}" write -field=token auth/approle/login role_id="$control_plane_role_id" secret_id="$control_plane_secret_id")"
api_provider_caps="$("${compose[@]}" exec -T -e BAO_ADDR=http://127.0.0.1:8200 -e "BAO_TOKEN=$api_token" openbao bao token capabilities cplane/data/platform/s3/providers/policy-smoke-test)"
control_plane_nonprovider_caps="$("${compose[@]}" exec -T -e BAO_ADDR=http://127.0.0.1:8200 -e "BAO_TOKEN=$control_plane_token" openbao bao token capabilities cplane/data/platform/s3/access-keys/policy-smoke-test)"
printf '%s' "$api_provider_caps" | grep -qw read
if printf '%s' "$api_provider_caps" | grep -Eqw 'create|update|delete'; then
  echo "API AppRole unexpectedly has provider write access" >&2
  exit 1
fi
[ "$control_plane_nonprovider_caps" = deny ] || { echo "Control-plane AppRole unexpectedly has non-provider access" >&2; exit 1; }
unset api_token control_plane_token api_secret_id control_plane_secret_id

echo "Applying database migrations..."
run_quiet "${compose[@]}" run --rm --build migrate
echo "Database migrations complete"

provider_id="$(env_value REGISTRY_STORAGE_S3_PROVIDER_ID)"
provider_exists=0
if [ -n "$provider_id" ]; then
  provider_exists="$("${compose[@]}" exec -T postgresd psql -U cplane -d cplane -At \
    -v provider_id="$provider_id" <<'SQL'
SELECT EXISTS (SELECT 1 FROM s3_providers WHERE id=:'provider_id'::uuid AND is_active=true)::int;
SQL
  )"
fi

if [ "$provider_exists" != 1 ]; then
  echo
  echo "Configure the first S3 provider"
  while :; do
    read -r -p "Provider type [aws_s3/cloudflare_r2] (aws_s3): " provider_type
    provider_type="${provider_type:-aws_s3}"
    case "$provider_type" in
      aws_s3) default_provider_region=us-east-1 ;;
      cloudflare_r2) default_provider_region=auto ;;
      *) echo "Provider type must be aws_s3 or cloudflare_r2; try again." >&2; continue ;;
    esac

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
  registry_bucket="$(env_value REGISTRY_STORAGE_S3_BUCKET)"
  echo "Registry backing bucket: $registry_bucket"

  provider_id="$(new_uuid)"
  region_id="$(new_uuid)"
  bao=("${compose[@]}" exec -T -e BAO_ADDR=http://127.0.0.1:8200 -e "BAO_TOKEN=$(env_value OPENBAO_ROOT_TOKEN)" openbao bao kv put "cplane/platform/s3/providers/$provider_id")
  if [ -n "$provider_session_token" ]; then
    run_quiet "${bao[@]}" "access_key_id=$provider_access_key" "secret_access_key=$provider_secret_key" "session_token=$provider_session_token"
  else
    run_quiet "${bao[@]}" "access_key_id=$provider_access_key" "secret_access_key=$provider_secret_key"
  fi

  if ! run_quiet "${compose[@]}" exec -T postgresd psql -v ON_ERROR_STOP=1 -U cplane -d cplane \
    -v provider_id="$provider_id" -v provider_type="$provider_type" \
    -v provider_endpoint="$provider_endpoint" -v provider_region="$provider_region" \
    -v region_id="$region_id" -v region_slug="$region_slug" -v region_name="$region_name" <<'SQL'
BEGIN;
INSERT INTO s3_providers (id, provider_type, endpoint_url, provider_region, is_active)
VALUES (:'provider_id'::uuid, :'provider_type'::s3_provider_type, :'provider_endpoint', :'provider_region', true);
INSERT INTO regions (id, slug, display_name, s3_provider_id, status, routing_mode)
VALUES (:'region_id'::uuid, :'region_slug', :'region_name', :'provider_id'::uuid, 'active', 'active');
INSERT INTO infrastructure_audit_log (id, actor_identifier, source_ip, action, resource_type, resource_id, changes)
VALUES
  (gen_random_uuid(), 'install.sh', 'local', 'create', 's3_provider', :'provider_id'::uuid, jsonb_build_object('provider_type', :'provider_type', 'endpoint_url', :'provider_endpoint', 'provider_region', :'provider_region', 'is_active', true)),
  (gen_random_uuid(), 'install.sh', 'local', 'create', 'region', :'region_id'::uuid, jsonb_build_object('slug', :'region_slug', 'display_name', :'region_name', 'status', 'active', 's3_provider_id', :'provider_id'));
COMMIT;
SQL
  then
    "${compose[@]}" exec -T -e BAO_ADDR=http://127.0.0.1:8200 -e "BAO_TOKEN=$(env_value OPENBAO_ROOT_TOKEN)" openbao \
      bao kv delete "cplane/platform/s3/providers/$provider_id" >/dev/null || true
    exit 1
  fi

  set_env REGISTRY_STORAGE_S3_PROVIDER_ID "$provider_id"
  set_env REGISTRY_STORAGE_S3_PHYSICAL_BUCKET "$registry_bucket"
  unset provider_secret_key provider_session_token
fi

echo "Bootstrapping registry storage..."
registry_storage_id="$(new_uuid)"
registry_storage_id="$("${compose[@]}" exec -T postgresd psql -v ON_ERROR_STOP=1 -U cplane -d cplane -At \
  -v storage_id="$registry_storage_id" \
  -v provider_id="$(env_value REGISTRY_STORAGE_S3_PROVIDER_ID)" \
  -v bucket_name="$(env_value REGISTRY_STORAGE_S3_BUCKET)" \
  -v physical_bucket_name="$(env_value REGISTRY_STORAGE_S3_PHYSICAL_BUCKET)" \
  -v access_key="$(env_value REGISTRY_STORAGE_S3_ACCESSKEY)" \
  -v gc_access_key="$(env_value REGISTRY_STORAGE_S3_GC_ACCESSKEY)" <<'SQL'
INSERT INTO registry_storage (id, service, provider_id, bucket_name, physical_bucket_name, access_key_id)
VALUES (:'storage_id'::uuid, 'distribution', :'provider_id'::uuid, :'bucket_name', :'physical_bucket_name', :'access_key')
ON CONFLICT (service) DO UPDATE SET provider_id=EXCLUDED.provider_id, bucket_name=EXCLUDED.bucket_name,
  physical_bucket_name=EXCLUDED.physical_bucket_name, access_key_id=EXCLUDED.access_key_id, updated_at=NOW()
RETURNING id;
INSERT INTO registry_maintenance (service, gc_access_key_id)
VALUES ('distribution', :'gc_access_key')
ON CONFLICT (service) DO UPDATE SET gc_access_key_id=EXCLUDED.gc_access_key_id, updated_at=NOW();
SQL
)"
registry_storage_id="$(printf '%s\n' "$registry_storage_id" | head -n 1)"
run_quiet "${root_bao[@]}" kv put "cplane/platform/s3/access-keys/$(env_value REGISTRY_STORAGE_S3_ACCESSKEY)" \
  kind=distribution "credential_id=$registry_storage_id" "secret_access_key=$(env_value REGISTRY_STORAGE_S3_SECRETKEY)"
run_quiet "${root_bao[@]}" kv put "cplane/platform/s3/access-keys/$(env_value REGISTRY_STORAGE_S3_GC_ACCESSKEY)" \
  kind=distribution_gc "credential_id=$registry_storage_id" "secret_access_key=$(env_value REGISTRY_STORAGE_S3_GC_SECRETKEY)"
if ! "${root_bao[@]}" kv get -field=key "cplane/storage/sse-c/$registry_storage_id" >/dev/null 2>&1; then
  run_quiet "${root_bao[@]}" kv put "cplane/storage/sse-c/$registry_storage_id" "key=$(openssl rand -base64 32 | tr -d '\n')"
fi

echo "Ensuring registry backing bucket..."
registry_provider_id="$(env_value REGISTRY_STORAGE_S3_PROVIDER_ID)"
registry_bucket="$(env_value REGISTRY_STORAGE_S3_PHYSICAL_BUCKET)"
registry_bucket="${registry_bucket:-$(env_value REGISTRY_STORAGE_S3_BUCKET)}"
registry_provider="$("${compose[@]}" exec -T postgresd psql -U cplane -d cplane -At -F $'\t' -v provider_id="$registry_provider_id" <<'SQL'
SELECT endpoint_url, provider_region, provider_type::text
FROM s3_providers
WHERE id=:'provider_id'::uuid AND is_active=true;
SQL
)"
IFS=$'\t' read -r registry_endpoint registry_region registry_provider_type <<< "$registry_provider"
if [ -z "$registry_endpoint" ] || [ -z "$registry_region" ] || [ -z "$registry_provider_type" ]; then
  echo "Registry S3 provider is not active" >&2
  exit 1
fi

bao=("${compose[@]}" exec -T -e BAO_ADDR=http://127.0.0.1:8200 -e "BAO_TOKEN=$(env_value OPENBAO_ROOT_TOKEN)" openbao bao kv get)
registry_access_key="$("${bao[@]}" -field=access_key_id "cplane/platform/s3/providers/$registry_provider_id")"
registry_secret_key="$("${bao[@]}" -field=secret_access_key "cplane/platform/s3/providers/$registry_provider_id")"
registry_session_token="$("${bao[@]}" -field=session_token "cplane/platform/s3/providers/$registry_provider_id" 2>/dev/null || true)"
registry_bucket_url="${registry_endpoint%/}/$registry_bucket"
s3=(curl --silent --show-error --aws-sigv4 "aws:amz:$registry_region:s3" --user "$registry_access_key:$registry_secret_key")
[ -z "$registry_session_token" ] || s3+=(-H "x-amz-security-token: $registry_session_token")
bucket_status="$("${s3[@]}" -o /dev/null -w '%{http_code}' --head "$registry_bucket_url" || true)"
if [ "$bucket_status" = 404 ]; then
  create=("${s3[@]}" --fail -X PUT)
  if [ "$registry_provider_type" = aws_s3 ] && [ "$registry_region" != us-east-1 ]; then
    create+=(-H 'Content-Type: application/xml' --data "<CreateBucketConfiguration xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\"><LocationConstraint>$registry_region</LocationConstraint></CreateBucketConfiguration>")
  fi
  "${create[@]}" "$registry_bucket_url"
elif [ "$bucket_status" != 200 ]; then
  echo "Unable to check registry backing bucket (S3 returned $bucket_status)" >&2
  exit 1
fi
unset registry_secret_key registry_session_token

services=(storage ui api registry control-plane worker)
echo "Starting C-Plane ($mode)..."
"${compose[@]}" up -d --build "${services[@]}"
echo "C-Plane is installed"
