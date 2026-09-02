use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::StatusCode;
use serde_json::{Value, json};
use std::{env, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Error)]
pub enum SecretError {
    #[error("missing environment variable {0}")]
    MissingEnv(&'static str),
    #[error("OpenBao request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("OpenBao returned {0}")]
    Status(StatusCode),
    #[error("invalid OpenBao response: {0}")]
    InvalidResponse(#[from] serde_json::Error),
}

pub const PLATFORM_KEY: &str = "platform";

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    address: String,
    token: Arc<RwLock<Option<String>>>,
    approle: Option<(String, String)>,
}

impl Client {
    pub fn with_token(
        address: impl Into<String>,
        token: impl Into<String>,
    ) -> Result<Self, SecretError> {
        Self::new(address.into(), Some(token.into()), None)
    }

    pub fn from_env() -> Result<Self, SecretError> {
        let address = required("OPENBAO_ADDR")?;
        let token = env::var("OPENBAO_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let approle = if token.is_some() {
            None
        } else {
            Some((required("OPENBAO_ROLE_ID")?, required("OPENBAO_SECRET_ID")?))
        };
        Self::new(address, token, approle)
    }

    fn new(
        address: String,
        token: Option<String>,
        approle: Option<(String, String)>,
    ) -> Result<Self, SecretError> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()?,
            address: address.trim_end_matches('/').to_owned(),
            token: Arc::new(RwLock::new(token)),
            approle,
        })
    }

    async fn post(&self, path: &str, body: Value) -> Result<reqwest::Response, SecretError> {
        for attempt in 0..2 {
            let token = self.token().await?;
            let url = format!("{}/v1/{path}", self.address);
            let mut request = self.http.post(url).header("X-Vault-Token", token);
            request = request.json(&body);
            let response = request.send().await?;
            if attempt == 0
                && self.approle.is_some()
                && matches!(
                    response.status(),
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
                )
            {
                *self.token.write().await = None;
                continue;
            }
            return Ok(response);
        }
        unreachable!()
    }

    async fn token(&self) -> Result<String, SecretError> {
        if let Some(token) = self.token.read().await.clone() {
            return Ok(token);
        }
        let (role_id, secret_id) = self
            .approle
            .as_ref()
            .expect("Client always has a token or AppRole credentials");
        let response = self
            .http
            .post(format!("{}/v1/auth/approle/login", self.address))
            .json(&json!({ "role_id": role_id, "secret_id": secret_id }))
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(SecretError::Status(response.status()));
        }
        let token = response.json::<LoginResponse>().await?.auth.client_token;
        *self.token.write().await = Some(token.clone());
        Ok(token)
    }
}

pub async fn encrypt(client: &Client, key: &str, plaintext: &[u8]) -> Result<String, SecretError> {
    let response = client
        .post(
            &format!("transit/encrypt/{key}"),
            json!({ "plaintext": STANDARD.encode(plaintext) }),
        )
        .await?;
    if !response.status().is_success() {
        return Err(SecretError::Status(response.status()));
    }
    Ok(response
        .json::<TransitEncryptResponse>()
        .await?
        .data
        .ciphertext)
}

pub async fn create_key(client: &Client, key: &str) -> Result<(), SecretError> {
    let response = client
        .post(&format!("transit/keys/{key}"), json!({}))
        .await?;
    if !response.status().is_success() {
        return Err(SecretError::Status(response.status()));
    }
    Ok(())
}

pub async fn decrypt(client: &Client, key: &str, ciphertext: &str) -> Result<Vec<u8>, SecretError> {
    let response = client
        .post(
            &format!("transit/decrypt/{key}"),
            json!({ "ciphertext": ciphertext }),
        )
        .await?;
    if !response.status().is_success() {
        return Err(SecretError::Status(response.status()));
    }
    let plaintext = response
        .json::<TransitDecryptResponse>()
        .await?
        .data
        .plaintext;
    STANDARD.decode(plaintext).map_err(|error| {
        SecretError::InvalidResponse(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error,
        )))
    })
}

fn required(name: &'static str) -> Result<String, SecretError> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or(SecretError::MissingEnv(name))
}

#[derive(serde::Deserialize)]
struct LoginResponse {
    auth: LoginAuth,
}

#[derive(serde::Deserialize)]
struct LoginAuth {
    client_token: String,
}

#[derive(serde::Deserialize)]
struct TransitEncryptResponse {
    data: TransitCiphertext,
}

#[derive(serde::Deserialize)]
struct TransitCiphertext {
    ciphertext: String,
}

#[derive(serde::Deserialize)]
struct TransitDecryptResponse {
    data: TransitPlaintext,
}

#[derive(serde::Deserialize)]
struct TransitPlaintext {
    plaintext: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, extract::State, http::HeaderMap, routing::any};
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Clone)]
    struct TestState {
        logins: Arc<AtomicUsize>,
        requests: Arc<Mutex<Vec<String>>>,
    }

    async fn handler(
        State(state): State<TestState>,
        headers: HeaderMap,
        request: axum::extract::Request,
    ) -> (StatusCode, Json<Value>) {
        let path = request.uri().path();
        if path == "/v1/auth/approle/login" {
            let login = state.logins.fetch_add(1, Ordering::SeqCst) + 1;
            return (
                StatusCode::OK,
                Json(json!({ "auth": { "client_token": format!("token-{login}") } })),
            );
        }
        if headers
            .get("x-vault-token")
            .is_none_or(|token| token == "token-1")
        {
            return (StatusCode::FORBIDDEN, Json(json!({})));
        }
        state.requests.lock().unwrap().push(path.to_owned());
        if path.starts_with("/v1/transit/keys/") {
            return (StatusCode::NO_CONTENT, Json(json!({})));
        }
        if path.starts_with("/v1/transit/encrypt/") {
            return (
                StatusCode::OK,
                Json(json!({ "data": { "ciphertext": "vault:v1:ciphertext" } })),
            );
        }
        if path.starts_with("/v1/transit/decrypt/") {
            if path.ends_with("/malformed") {
                return (
                    StatusCode::OK,
                    Json(json!({ "data": { "plaintext": "%%%" } })),
                );
            }
            return (
                StatusCode::OK,
                Json(json!({ "data": { "plaintext": STANDARD.encode(b"decrypted") } })),
            );
        }
        (StatusCode::METHOD_NOT_ALLOWED, Json(json!({})))
    }

    #[tokio::test]
    async fn encrypts_and_decrypts() {
        let state = TestState {
            logins: Arc::new(AtomicUsize::new(0)),
            requests: Arc::new(Mutex::new(Vec::new())),
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(
            axum::serve(
                listener,
                Router::new()
                    .fallback(any(handler))
                    .with_state(state.clone()),
            )
            .into_future(),
        );
        let secrets = Client::new(
            format!("http://{address}"),
            None,
            Some(("role".into(), "secret".into())),
        )
        .unwrap();

        create_key(&secrets, "tenant-key").await.unwrap();
        let ciphertext = encrypt(&secrets, "tenant-key", b"plaintext").await.unwrap();
        assert_eq!(ciphertext, "vault:v1:ciphertext");
        assert_eq!(
            decrypt(&secrets, "tenant-key", &ciphertext).await.unwrap(),
            b"decrypted"
        );
        assert!(decrypt(&secrets, "malformed", &ciphertext).await.is_err());
        assert_eq!(state.logins.load(Ordering::SeqCst), 2);
        assert_eq!(
            *state.requests.lock().unwrap(),
            [
                "/v1/transit/keys/tenant-key",
                "/v1/transit/encrypt/tenant-key",
                "/v1/transit/decrypt/tenant-key",
                "/v1/transit/decrypt/malformed",
            ]
        );
        server.abort();
    }
}
