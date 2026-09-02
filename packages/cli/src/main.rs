use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use lib::{
    buckets,
    entities::{
        bucket, bucket_grant, clickhouse_provider, credential, region, s3_provider, secret,
    },
    provisioning::provision_platform_bucket,
    secrets::{self, Client as SecretsClient, PLATFORM_KEY},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, Set, Statement, TransactionTrait,
};
use serde::Serialize;
use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};
use uuid::Uuid;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

const SERVICES: &[&str] = &[
    "ingress",
    "storage",
    "ui",
    "api",
    "registry",
    "control-plane",
    "worker",
    "clickhouse",
];

#[derive(Clone, Copy)]
enum Mode {
    Dev,
    Prod,
}

impl Mode {
    fn parse(value: Option<&str>) -> Result<Self> {
        match value.unwrap_or("prod") {
            "dev" => Ok(Self::Dev),
            "prod" => Ok(Self::Prod),
            value => Err(format!("invalid mode '{value}'; usage: build.sh [dev|prod]").into()),
        }
    }

    fn compose_file(self) -> &'static str {
        match self {
            Self::Dev => "docker-compose.dev.yml",
            Self::Prod => "docker-compose.prod.yml",
        }
    }
}

struct EnvFile {
    path: PathBuf,
    lines: Vec<String>,
}

impl EnvFile {
    fn load(repo: &Path) -> Result<Self> {
        let path = repo.join(".env");
        if !path.exists() {
            fs::copy(repo.join(".env.example"), &path)?;
        }
        let contents = fs::read_to_string(&path)?;
        Ok(Self {
            path,
            lines: contents.lines().map(str::to_owned).collect(),
        })
    }

    fn get(&self, name: &str) -> Option<String> {
        self.lines
            .iter()
            .filter_map(|line| {
                let (key, value) = line.split_once('=')?;
                if key.trim() != name {
                    return None;
                }
                Some(value.trim().trim_matches(['"', '\'']).to_owned())
            })
            .last()
    }

    fn set(&mut self, name: &str, value: &str) {
        let mut found = false;
        for line in &mut self.lines {
            if line
                .split_once('=')
                .is_some_and(|(key, _)| key.trim() == name)
            {
                *line = format!("{name}={value}");
                found = true;
            }
        }
        if !found {
            if !self.lines.is_empty() && !self.lines.last().is_some_and(|line| line.is_empty()) {
                self.lines.push(String::new());
            }
            self.lines.push(format!("{name}={value}"));
        }
    }

    fn required(&self, name: &str) -> Result<String> {
        self.get(name)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("{name} is required").into())
    }

    fn save(&self) -> Result<()> {
        fs::write(&self.path, format!("{}\n", self.lines.join("\n")))?;
        Ok(())
    }
}

struct Compose {
    repo: PathBuf,
    env_file: PathBuf,
    compose_file: String,
}

impl Compose {
    fn command<I, S>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let mut command = Command::new("docker");
        command
            .current_dir(&self.repo)
            .arg("compose")
            .arg("--env-file")
            .arg(&self.env_file)
            .arg("-f")
            .arg(&self.compose_file)
            .args(args);
        command
    }

    fn run<I, S>(&self, label: &str, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S> + Clone,
        S: AsRef<std::ffi::OsStr>,
    {
        println!("{label}...");
        let status = self.command(args).status()?;
        if status.success() {
            println!("{label} done");
            Ok(())
        } else {
            Err(format!("{label} failed with {status}").into())
        }
    }

    fn output<I, S>(&self, args: I) -> Result<Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        Ok(self.command(args).output()?)
    }

    fn capture<I, S>(&self, args: I) -> Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let output = self.output(args)?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr)
                .trim()
                .to_owned()
                .into());
        }
        Ok(String::from_utf8(output.stdout)?.trim().to_owned())
    }

    fn capture_with_stdin<I, S>(&self, args: I, input: &Path) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let mut command = self.command(args);
        command.stdin(Stdio::from(File::open(input)?));
        let status = command.status()?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("command failed with {status}").into())
        }
    }
}

#[derive(Serialize)]
struct S3ProviderCredentials {
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
}

#[derive(Serialize)]
struct ClickHouseCredentials {
    username: String,
    password: String,
}

fn random_bytes_hex(bytes: usize) -> String {
    let mut value = String::with_capacity(bytes * 2);
    while value.len() < bytes * 2 {
        value.push_str(&Uuid::new_v4().simple().to_string());
    }
    value.truncate(bytes * 2);
    value
}

fn random_token() -> String {
    URL_SAFE_NO_PAD.encode(random_bytes_hex(32).as_bytes())
}

fn ensure_secret(env_file: &mut EnvFile, name: &str) {
    let value = env_file.get(name).unwrap_or_default();
    if value.is_empty()
        || value.starts_with("mysecret")
        || value.starts_with("your-secure-")
        || value.starts_with("replace-with-")
        || value.starts_with("generated-by-")
    {
        env_file.set(name, &random_bytes_hex(32));
    }
}

fn configure_defaults(env_file: &mut EnvFile) {
    for name in [
        "POSTGRES_PASSWORD",
        "POSTGRES_UI_PASSWORD",
        "POSTGRES_IDENTITY_PASSWORD",
        "POSTGRES_TENANT_PASSWORD",
        "POSTGRES_ADMIN_PASSWORD",
        "VALKEY_PASSWORD",
        "BETTER_AUTH_SECRET",
        "CPLANE_SERVICE_TOKEN",
        "REGISTRY_HTTP_SECRET",
        "CLICKHOUSE_PASSWORD",
        "CLICKHOUSE_STORAGE_SECRET_ACCESS_KEY",
    ] {
        ensure_secret(env_file, name);
    }
    if env_file
        .get("CLICKHOUSE_STORAGE_ACCESS_KEY_ID")
        .is_none_or(|value| value.is_empty() || value.starts_with("generated-by-"))
    {
        env_file.set(
            "CLICKHOUSE_STORAGE_ACCESS_KEY_ID",
            &format!("CP{}", random_bytes_hex(16).to_uppercase()),
        );
    }
    for (name, value) in [
        ("CLICKHOUSE_DB", "cplane"),
        ("CLICKHOUSE_USER", "cplane"),
        ("CLICKHOUSE_PROVIDER_NAME", "Bundled ClickHouse"),
        ("CLICKHOUSE_ENDPOINT_URL", "http://clickhouse:8123"),
        ("CLICKHOUSE_CLUSTER_NAME", "cplane"),
        ("REGISTRY_HOST", "localhost:5000"),
    ] {
        if env_file.get(name).is_none_or(|value| value.is_empty()) {
            env_file.set(name, value);
        }
    }
    if env_file
        .get("REGISTRY_TOKEN_SECRET")
        .is_none_or(|value| value.is_empty() || value.starts_with("generated-by-"))
    {
        env_file.set("REGISTRY_TOKEN_SECRET", &random_token());
    }
    if env_file
        .get("REGISTRY_TOKEN_REALM")
        .is_none_or(|value| value == "http://localhost:3000/api/backend/registry/token")
    {
        env_file.set(
            "REGISTRY_TOKEN_REALM",
            "http://localhost:8080/api/registry/token",
        );
    }
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim().to_owned())
}

fn prompt_secret(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush()?;
    let hidden = io::stdin().is_terminal() && (cfg!(unix) || env::var_os("MSYSTEM").is_some());
    let value = if hidden {
        let _ = Command::new("stty").arg("-echo").status();
        let mut value = String::new();
        io::stdin().read_line(&mut value)?;
        let _ = Command::new("stty").arg("echo").status();
        value.trim().to_owned()
    } else {
        let mut value = String::new();
        io::stdin().read_line(&mut value)?;
        value.trim().to_owned()
    };
    println!();
    Ok(value)
}

fn valid_domain(value: &str) -> bool {
    !value.is_empty()
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(|byte| byte.is_ascii_alphanumeric())
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(|byte| byte.is_ascii_alphanumeric())
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

fn configure_ingress(mode: Mode, env_file: &mut EnvFile) -> Result<()> {
    let use_domain = prompt("Use a public domain for ingress? [y/N]: ")?;
    if matches!(use_domain.as_str(), "y" | "Y" | "yes" | "YES" | "Yes") {
        let current = env_file.get("CPLANE_DOMAIN").unwrap_or_default();
        let entered = if current.is_empty() {
            prompt("Public domain (for example: example.com): ")?
        } else {
            let value = prompt(&format!("Public domain [{current}]: "))?;
            if value.is_empty() { current } else { value }
        };
        if !valid_domain(&entered) {
            return Err(
                "the domain must be a hostname without a scheme, port, path, or trailing dot"
                    .into(),
            );
        }
        env_file.set("CPLANE_DOMAIN", &entered);
        env_file.set("NUXT_AUTH_BASE_URL", &format!("https://{entered}"));
        env_file.set("INGRESS_PLATFORM_HOSTS", &entered);
        env_file.set("INGRESS_API_HOSTS", &format!("api.{entered}"));
        env_file.set("INGRESS_STORAGE_HOSTS", &format!("storage.{entered}"));
        env_file.set("INGRESS_REGISTRY_HOSTS", &format!("registry.{entered}"));
        env_file.set("INGRESS_FORWARDED_PROTO", "https");
        env_file.set("REGISTRY_HOST", &format!("registry.{entered}"));
        env_file.set("REGISTRY_INTERNAL_URL", "http://registry:5000");
        env_file.set(
            "REGISTRY_TOKEN_REALM",
            &format!("https://api.{entered}/api/registry/token"),
        );
    } else {
        if matches!(mode, Mode::Prod) {
            return Err("production requires a public domain for ingress".into());
        }
        for (name, value) in [
            ("NUXT_AUTH_BASE_URL", "http://localhost:3000"),
            ("INGRESS_PLATFORM_HOSTS", "localhost:3000"),
            ("INGRESS_API_HOSTS", "localhost:8080"),
            ("INGRESS_STORAGE_HOSTS", "localhost:8081"),
            ("INGRESS_REGISTRY_HOSTS", "localhost:5000"),
            ("INGRESS_FORWARDED_PROTO", "http"),
            ("REGISTRY_HOST", "localhost:5000"),
            ("REGISTRY_INTERNAL_URL", "http://registry:5000"),
            (
                "REGISTRY_TOKEN_REALM",
                "http://localhost:8080/api/registry/token",
            ),
        ] {
            env_file.set(name, value);
        }
    }
    Ok(())
}

fn required_value(value: Option<String>, name: &str) -> Result<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} is required").into())
}

fn validate_region_slug(value: String) -> Result<String> {
    let value = value.to_ascii_lowercase();
    if value.is_empty()
        || matches!(value.as_str(), "default" | "global" | "system")
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err("invalid or reserved region slug".into());
    }
    Ok(value)
}

fn run_docker_checks() -> Result<()> {
    for command in ["docker"] {
        if Command::new(command)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_err()
        {
            return Err(format!("missing required command: {command}").into());
        }
    }
    let status = Command::new("docker")
        .args(["compose", "version"])
        .status()?;
    if !status.success() {
        return Err("docker compose is unavailable".into());
    }
    Ok(())
}

fn bao_args(token: &str, args: &[String]) -> Vec<String> {
    let mut values = vec![
        "exec".into(),
        "-T".into(),
        "-e".into(),
        "BAO_ADDR=http://127.0.0.1:8200".into(),
        "-e".into(),
        format!("BAO_TOKEN={token}"),
        "openbao".into(),
        "bao".into(),
    ];
    values.extend(args.iter().cloned());
    values
}

fn configure_openbao(compose: &Compose, env_file: &mut EnvFile) -> Result<String> {
    let status = compose.output([
        "exec",
        "-T",
        "-e",
        "BAO_ADDR=http://127.0.0.1:8200",
        "openbao",
        "bao",
        "status",
        "-format=json",
    ])?;
    let initialized = String::from_utf8_lossy(&status.stdout)
        .contains("\"initialized\"        : true")
        || String::from_utf8_lossy(&status.stdout).contains("\"initialized\":true")
        || String::from_utf8_lossy(&status.stdout).contains("\"initialized\": true");
    let root_token = if initialized {
        let unseal_key = required_value(env_file.get("OPENBAO_UNSEAL_KEY"), "OPENBAO_UNSEAL_KEY")?;
        let root_token = required_value(env_file.get("OPENBAO_ROOT_TOKEN"), "OPENBAO_ROOT_TOKEN")?;
        if unseal_key.starts_with("generated-by-") || root_token.starts_with("generated-by-") {
            return Err(
                "OpenBao is already initialized, but .env does not contain its unseal key and root token"
                    .into(),
            );
        }
        root_token
    } else {
        let output = compose.capture([
            "exec",
            "-T",
            "-e",
            "BAO_ADDR=http://127.0.0.1:8200",
            "openbao",
            "bao",
            "operator",
            "init",
            "-key-shares=1",
            "-key-threshold=1",
        ])?;
        let unseal_key = output
            .lines()
            .find_map(|line| line.strip_prefix("Unseal Key 1: "))
            .map(str::to_owned);
        let root_token = output
            .lines()
            .find_map(|line| line.strip_prefix("Initial Root Token: "))
            .map(str::to_owned);
        let unseal_key = required_value(unseal_key, "OpenBao unseal key")?;
        let root_token = required_value(root_token, "OpenBao root token")?;
        env_file.set("OPENBAO_UNSEAL_KEY", &unseal_key);
        env_file.set("OPENBAO_ROOT_TOKEN", &root_token);
        env_file.save()?;
        root_token
    };

    compose.run(
        "Unsealing OpenBao and enabling Transit",
        ["run", "--rm", "openbao-init"],
    )?;
    let root = |args: &[&str]| -> Result<String> {
        let args = args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
        compose.capture(bao_args(&root_token, &args))
    };
    let _ = root(&["write", "-f", "transit/keys/platform"])?;
    let _ = compose.output(bao_args(
        &root_token,
        &["auth".into(), "enable".into(), "approle".into()],
    ))?;
    for policy in ["api", "control-plane", "worker"] {
        let mut args = vec![
            "policy".into(),
            "write".into(),
            format!("cplane-{policy}"),
            "-".into(),
        ];
        let policy_path = compose
            .repo
            .join(format!("packages/openbao/policies/{policy}.hcl"));
        compose.capture_with_stdin(bao_args(&root_token, &args), &policy_path)?;
        args.clear();
    }
    for (role, policy) in [
        ("api", "cplane-api"),
        ("control-plane", "cplane-control-plane"),
        ("worker", "cplane-worker"),
    ] {
        let role_name = format!("cplane-{role}");
        let _ = root(&[
            "write",
            &format!("auth/approle/role/{role_name}"),
            &format!("token_policies={policy}"),
            "token_ttl=1h",
            "token_max_ttl=4h",
        ])?;
    }
    let api_role_id = root(&[
        "read",
        "-field=role_id",
        "auth/approle/role/cplane-api/role-id",
    ])?;
    let api_secret_id = root(&[
        "write",
        "-field=secret_id",
        "-f",
        "auth/approle/role/cplane-api/secret-id",
    ])?;
    let control_role_id = root(&[
        "read",
        "-field=role_id",
        "auth/approle/role/cplane-control-plane/role-id",
    ])?;
    let control_secret_id = root(&[
        "write",
        "-field=secret_id",
        "-f",
        "auth/approle/role/cplane-control-plane/secret-id",
    ])?;
    let worker_role_id = root(&[
        "read",
        "-field=role_id",
        "auth/approle/role/cplane-worker/role-id",
    ])?;
    let worker_secret_id = root(&[
        "write",
        "-field=secret_id",
        "-f",
        "auth/approle/role/cplane-worker/secret-id",
    ])?;
    for (name, value) in [
        ("OPENBAO_API_ROLE_ID", api_role_id.clone()),
        ("OPENBAO_API_SECRET_ID", api_secret_id.clone()),
        ("OPENBAO_CONTROL_PLANE_ROLE_ID", control_role_id.clone()),
        ("OPENBAO_CONTROL_PLANE_SECRET_ID", control_secret_id.clone()),
        ("OPENBAO_WORKER_ROLE_ID", worker_role_id),
        ("OPENBAO_WORKER_SECRET_ID", worker_secret_id),
    ] {
        env_file.set(name, &value);
    }
    let api_token = root(&[
        "write",
        "-field=token",
        "auth/approle/login",
        &format!("role_id={api_role_id}"),
        &format!("secret_id={api_secret_id}"),
    ])?;
    let control_token = root(&[
        "write",
        "-field=token",
        "auth/approle/login",
        &format!("role_id={control_role_id}"),
        &format!("secret_id={control_secret_id}"),
    ])?;
    let api_caps = compose.capture(bao_args(
        &api_token,
        &[
            "token".into(),
            "capabilities".into(),
            "transit/decrypt/platform".into(),
        ],
    ))?;
    let control_caps = compose.capture(bao_args(
        &control_token,
        &[
            "token".into(),
            "capabilities".into(),
            "transit/encrypt/tenant-policy-smoke-test".into(),
        ],
    ))?;
    if !api_caps.split_whitespace().any(|value| value == "update") || control_caps != "deny" {
        return Err("OpenBao AppRole capability verification failed".into());
    }
    Ok(root_token)
}

async fn provision_tenant_keys(
    database: &DatabaseConnection,
    secrets_client: &SecretsClient,
) -> Result<()> {
    let rows = database
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT id FROM organization",
        ))
        .await?;
    for row in rows {
        let organization_id: Uuid = row.try_get("", "id")?;
        secrets::create_key(
            secrets_client,
            &format!("tenant-{}", organization_id.simple()),
        )
        .await?;
    }
    Ok(())
}

async fn audit<C: ConnectionTrait>(
    connection: &C,
    action: &str,
    resource_type: &str,
    resource_id: Uuid,
    changes: serde_json::Value,
) -> Result<()> {
    connection.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO infrastructure_audit_log (id, actor_identifier, source_ip, action, resource_type, resource_id, changes) VALUES ($1::uuid, 'local', 'local', $2, $3, $4::uuid, $5::jsonb)",
        vec![Uuid::new_v4().into(), action.into(), resource_type.into(), resource_id.into(), changes.to_string().into()],
    )).await?;
    Ok(())
}

async fn bootstrap_infrastructure(
    database: &DatabaseConnection,
    secrets_client: &SecretsClient,
    env_file: &EnvFile,
) -> Result<()> {
    let s3_count = s3_provider::Entity::find().count(database).await?;
    let region_count = region::Entity::find().count(database).await?;
    let clickhouse_count = clickhouse_provider::Entity::find().count(database).await?;
    if s3_count > 0 && region_count > 0 && clickhouse_count > 0 {
        for provider in clickhouse_provider::Entity::find().all(database).await? {
            let bucket_exists = bucket::Entity::find_by_id(provider.bucket_id)
                .one(database)
                .await?
                .is_some();
            let credential_exists = credential::Entity::find_by_id(provider.storage_credential_id)
                .one(database)
                .await?
                .is_some();
            let grant_exists = bucket_grant::Entity::find()
                .filter(bucket_grant::Column::CredentialId.eq(provider.storage_credential_id))
                .filter(bucket_grant::Column::BucketId.eq(provider.bucket_id))
                .filter(bucket_grant::Column::OrganizationId.is_null())
                .one(database)
                .await?
                .is_some();
            if !bucket_exists || !credential_exists || !grant_exists {
                return Err("partial infrastructure detected: an existing ClickHouse provider is missing its bucket, storage credential, or grant".into());
            }
        }
        println!("Infrastructure bootstrap already complete; nothing to do");
        return Ok(());
    }
    if s3_count != 0 || region_count != 0 || clickhouse_count != 0 {
        return Err(format!("partial infrastructure detected (S3 providers: {s3_count}, regions: {region_count}, ClickHouse providers: {clickhouse_count})").into());
    }

    println!("Configure the first S3 provider");
    let provider_name = required_value(Some(prompt("S3 provider name: ")?), "S3 provider name")?;
    let provider_endpoint = required_value(Some(prompt("S3 endpoint URL: ")?), "S3 endpoint URL")?;
    let provider_region = {
        let value = prompt("S3 signing region (us-east-1): ")?;
        if value.is_empty() {
            "us-east-1".to_owned()
        } else {
            value
        }
    };
    let provider_access_key = required_value(Some(prompt("Access key ID: ")?), "Access key ID")?;
    let provider_secret_key = required_value(
        Some(prompt_secret("Secret access key: ")?),
        "Secret access key",
    )?;
    let provider_session_token =
        prompt_secret("Session token (input hidden; press Enter to skip): ")?;
    let provider_session_token =
        (!provider_session_token.is_empty()).then_some(provider_session_token);
    let region_slug_input = prompt(&format!(
        "Default C-Plane region slug ({provider_region}): "
    ))?;
    let region_slug = validate_region_slug(if region_slug_input.is_empty() {
        provider_region.clone()
    } else {
        region_slug_input
    })?;
    let region_name_input = prompt(&format!("Default C-Plane region name ({region_slug}): "))?;
    let region_name = if region_name_input.is_empty() {
        region_slug.clone()
    } else {
        region_name_input
    };

    let provider_id = Uuid::new_v4();
    let provider_secret_id = Uuid::new_v4();
    let region_id = Uuid::new_v4();
    let clickhouse_id = Uuid::new_v4();
    let provider_ciphertext = secrets::encrypt(
        secrets_client,
        PLATFORM_KEY,
        &serde_json::to_vec(&S3ProviderCredentials {
            access_key_id: provider_access_key,
            secret_access_key: provider_secret_key,
            session_token: provider_session_token,
        })?,
    )
    .await?;
    let tx = database.begin().await?;
    secret::ActiveModel {
        id: Set(provider_secret_id),
        scope: Set(secret::SecretScope::Platform),
        organization_id: Set(None),
        ciphertext: Set(provider_ciphertext),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    s3_provider::ActiveModel {
        id: Set(provider_id),
        name: Set(provider_name.clone()),
        endpoint_url: Set(provider_endpoint.clone()),
        provider_region: Set(provider_region.clone()),
        credential_secret_id: Set(provider_secret_id),
        is_active: Set(true),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    let storage = provision_platform_bucket(
        &tx,
        secrets_client,
        provider_id,
        Some((
            env_file.required("CLICKHOUSE_STORAGE_ACCESS_KEY_ID")?,
            env_file.required("CLICKHOUSE_STORAGE_SECRET_ACCESS_KEY")?,
        )),
    )
    .await?;
    let result: Result<()> = async {
        let clickhouse_ciphertext = secrets::encrypt(secrets_client, PLATFORM_KEY, &serde_json::to_vec(&ClickHouseCredentials { username: env_file.required("CLICKHOUSE_USER")?, password: env_file.required("CLICKHOUSE_PASSWORD")? })?).await?;
        let clickhouse_secret_id = Uuid::new_v4();
        secret::ActiveModel { id: Set(clickhouse_secret_id), scope: Set(secret::SecretScope::Platform), organization_id: Set(None), ciphertext: Set(clickhouse_ciphertext), ..Default::default() }.insert(&tx).await?;
        clickhouse_provider::ActiveModel { id: Set(clickhouse_id), name: Set(env_file.required("CLICKHOUSE_PROVIDER_NAME")?), endpoint_url: Set(env_file.required("CLICKHOUSE_ENDPOINT_URL")?), cluster_name: Set(env_file.required("CLICKHOUSE_CLUSTER_NAME")?), credential_secret_id: Set(clickhouse_secret_id), bucket_id: Set(storage.bucket_id), storage_credential_id: Set(storage.storage_credential.id), ..Default::default() }.insert(&tx).await?;
        region::ActiveModel { id: Set(region_id), slug: Set(region_slug.clone()), display_name: Set(region_name.clone()), s3_provider_id: Set(Some(provider_id)), clickhouse_provider_id: Set(clickhouse_id), status: Set(region::RegionStatus::Active), routing_mode: Set(region::RegionRoutingMode::Active), ..Default::default() }.insert(&tx).await?;
        audit(&tx, "create", "s3_provider", provider_id, serde_json::json!({"name": provider_name, "endpoint_url": provider_endpoint, "provider_region": provider_region, "is_active": true})).await?;
        audit(&tx, "create", "region", region_id, serde_json::json!({"slug": region_slug, "display_name": region_name, "status": "active", "s3_provider_id": provider_id, "clickhouse_provider_id": clickhouse_id})).await?;
        audit(&tx, "create", "clickhouse_provider", clickhouse_id, serde_json::json!({"name": env_file.required("CLICKHOUSE_PROVIDER_NAME")?, "endpoint_url": env_file.required("CLICKHOUSE_ENDPOINT_URL")?, "cluster_name": env_file.required("CLICKHOUSE_CLUSTER_NAME")?, "s3_provider_id": provider_id})).await?;
        tx.commit().await?;
        Ok(())
    }.await;
    if let Err(error) = result {
        let _ = buckets::delete(&storage.provider, storage.bucket_id).await;
        return Err(error);
    }
    println!(
        "Infrastructure bootstrap created the initial S3 provider, region, ClickHouse provider, bucket, credential, and assignment"
    );
    Ok(())
}

fn database_url(env_file: &EnvFile) -> Result<String> {
    let password = env_file.required("POSTGRES_ADMIN_PASSWORD")?;
    Ok(format!(
        "postgresql://cplane_admin:{}@127.0.0.1:5432/cplane",
        percent_encode(&password)
    ))
}

fn percent_encode(value: &str) -> String {
    value.bytes().fold(String::new(), |mut encoded, byte| {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
        encoded
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let mode = Mode::parse(env::args().nth(1).as_deref())?;
    run_docker_checks()?;
    let repo = env::current_dir()?;
    let mut env_file = EnvFile::load(&repo)?;
    configure_ingress(mode, &mut env_file)?;
    configure_defaults(&mut env_file);
    env_file.save()?;
    let compose = Compose {
        repo: repo.clone(),
        env_file: PathBuf::from(".env"),
        compose_file: mode.compose_file().to_owned(),
    };
    compose.run(
        "Starting Postgres, Valkey, and OpenBao",
        ["up", "-d", "--wait", "postgresd", "valkey", "openbao"],
    )?;
    let root_token = configure_openbao(&compose, &mut env_file)?;
    env_file.save()?;
    compose.run(
        "Applying database migrations",
        ["run", "--rm", "--build", "migrate"],
    )?;
    let secrets_client = SecretsClient::with_token("http://127.0.0.1:8200", root_token)?;
    let database = Database::connect(database_url(&env_file)?).await?;
    provision_tenant_keys(&database, &secrets_client).await?;
    bootstrap_infrastructure(&database, &secrets_client, &env_file).await?;
    compose.run(
        "Starting C-Plane",
        std::iter::once("up")
            .chain(std::iter::once("-d"))
            .chain(std::iter::once("--build"))
            .chain(std::iter::once("--wait"))
            .chain(SERVICES.iter().copied()),
    )?;
    println!("C-Plane is installed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_install_modes() {
        assert!(matches!(Mode::parse(None).unwrap(), Mode::Prod));
        assert!(matches!(Mode::parse(Some("dev")).unwrap(), Mode::Dev));
        assert!(Mode::parse(Some("test")).is_err());
    }

    #[test]
    fn updates_env_values_without_duplicate_keys() {
        let mut env_file = EnvFile {
            path: PathBuf::from(".env"),
            lines: vec!["TOKEN=old".into(), "OTHER=value".into()],
        };
        env_file.set("TOKEN", "new");
        env_file.set("ADDED", "value");
        assert_eq!(env_file.get("TOKEN").as_deref(), Some("new"));
        assert_eq!(
            env_file
                .lines
                .iter()
                .filter(|line| line.starts_with("TOKEN="))
                .count(),
            1
        );
        assert_eq!(env_file.get("ADDED").as_deref(), Some("value"));
    }

    #[test]
    fn generates_install_defaults_and_validates_regions() {
        let mut env_file = EnvFile {
            path: PathBuf::from(".env"),
            lines: Vec::new(),
        };
        configure_defaults(&mut env_file);
        assert_eq!(env_file.get("CLICKHOUSE_DB").as_deref(), Some("cplane"));
        assert!(env_file.required("CLICKHOUSE_PASSWORD").unwrap().len() >= 32);
        assert_eq!(
            validate_region_slug("EU-NORTH-1".into()).unwrap(),
            "eu-north-1"
        );
        assert!(validate_region_slug("global".into()).is_err());
    }

    #[test]
    fn validates_domains() {
        assert!(valid_domain("example.com"));
        assert!(valid_domain("app.example-1.com"));
        assert!(!valid_domain("https://example.com"));
        assert!(!valid_domain("example.com/"));
    }

    #[test]
    fn encodes_database_passwords_for_urls() {
        assert_eq!(percent_encode("p@ss:word"), "p%40ss%3Aword");
    }
}
