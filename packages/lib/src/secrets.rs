use reqwest::{Method, StatusCode};
use serde::{Serialize, de::DeserializeOwned};
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

#[derive(Clone)]
pub struct Secrets {
    http: reqwest::Client,
    address: String,
    token: Arc<RwLock<Option<String>>>,
    approle: Option<(String, String)>,
}

impl Secrets {
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

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>, SecretError> {
        let response = self.request(Method::GET, path, None).await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(SecretError::Status(response.status()));
        }
        let body: KvResponse<T> = response.json().await?;
        Ok(Some(body.data.data))
    }

    pub async fn set<T: Serialize>(&self, path: &str, value: &T) -> Result<(), SecretError> {
        let response = self
            .request(Method::POST, path, Some(json!({ "data": value })))
            .await?;
        if !response.status().is_success() {
            return Err(SecretError::Status(response.status()));
        }
        Ok(())
    }

    pub async fn delete(&self, path: &str) -> Result<(), SecretError> {
        let response = self.request(Method::DELETE, path, None).await?;
        if !response.status().is_success() && response.status() != StatusCode::NOT_FOUND {
            return Err(SecretError::Status(response.status()));
        }
        Ok(())
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<reqwest::Response, SecretError> {
        for attempt in 0..2 {
            let token = self.token().await?;
            let url = if method == Method::DELETE {
                format!("{}/v1/cplane/metadata/{path}", self.address)
            } else {
                format!("{}/v1/cplane/data/{path}", self.address)
            };
            let mut request = self
                .http
                .request(method.clone(), url)
                .header("X-Vault-Token", token);
            if let Some(body) = body.as_ref() {
                request = request.json(body);
            }
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
            .expect("Secrets always has a token or AppRole credentials");
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
struct KvResponse<T> {
    data: KvData<T>,
}

#[derive(serde::Deserialize)]
struct KvData<T> {
    data: T,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, extract::State, http::HeaderMap, routing::any};
    use serde::{Deserialize, Serialize};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Clone)]
    struct TestState {
        logins: Arc<AtomicUsize>,
    }

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct TestSecret {
        value: String,
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
        match *request.method() {
            Method::GET if path.ends_with("/missing") => (StatusCode::NOT_FOUND, Json(json!({}))),
            Method::DELETE if path.ends_with("/missing-delete") => {
                (StatusCode::NOT_FOUND, Json(json!({})))
            }
            Method::GET => (
                StatusCode::OK,
                Json(json!({ "data": { "data": { "value": "stored" } } })),
            ),
            Method::POST | Method::DELETE => (StatusCode::NO_CONTENT, Json(json!({}))),
            _ => (StatusCode::METHOD_NOT_ALLOWED, Json(json!({}))),
        }
    }

    #[tokio::test]
    async fn gets_sets_deletes_and_reauthenticates_once() {
        let state = TestState {
            logins: Arc::new(AtomicUsize::new(0)),
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
        let secrets = Secrets::new(
            format!("http://{address}"),
            None,
            Some(("role".into(), "secret".into())),
        )
        .unwrap();

        assert_eq!(
            secrets.get::<TestSecret>("present").await.unwrap(),
            Some(TestSecret {
                value: "stored".into()
            })
        );
        assert_eq!(state.logins.load(Ordering::SeqCst), 2);
        assert_eq!(secrets.get::<TestSecret>("missing").await.unwrap(), None);
        secrets
            .set(
                "present",
                &TestSecret {
                    value: "new".into(),
                },
            )
            .await
            .unwrap();
        secrets.delete("present").await.unwrap();
        secrets.delete("missing-delete").await.unwrap();
        server.abort();
    }
}
