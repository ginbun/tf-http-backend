use std::{collections::HashMap, sync::Arc};

use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{header, HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::any,
    Json, Router,
};
use base64::Engine;
use serde_json::{json, Value};

use crate::{
    config::BasicAuth,
    store::{AcquireLockResult, ReleaseLockResult, StateStore},
};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn StateStore>,
    pub basic_auth: Option<BasicAuth>,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", any(handle_request))
        .route("/{*path}", any(handle_request))
        .with_state(state)
}

async fn handle_request(
    State(app_state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    if !is_authorized(&headers, &app_state.basic_auth) {
        return unauthorized_response();
    }

    let resource_key = uri.path().to_string();
    let lock_id = query
        .get("ID")
        .or_else(|| query.get("id"))
        .map(String::as_str);

    match method.as_str() {
        "GET" => match app_state.store.get_state(&resource_key).await {
            Ok(Some(state)) => (StatusCode::OK, Json(state)).into_response(),
            Ok(None) => StatusCode::NOT_FOUND.into_response(),
            Err(_) => internal_error_response(),
        },
        "POST" | "PUT" | "PATCH" => {
            if let Some(resp) = ensure_lock_allows_write(&app_state, &resource_key, lock_id).await {
                return resp;
            }

            let parsed = parse_json_body(&body);
            let Ok(state) = parsed else {
                return bad_request("state payload must be valid JSON");
            };

            match app_state.store.upsert_state(&resource_key, &state).await {
                Ok(()) => StatusCode::OK.into_response(),
                Err(_) => internal_error_response(),
            }
        }
        "DELETE" => {
            if let Some(resp) = ensure_lock_allows_write(&app_state, &resource_key, lock_id).await {
                return resp;
            }

            match app_state.store.delete_state(&resource_key).await {
                Ok(()) => StatusCode::OK.into_response(),
                Err(_) => internal_error_response(),
            }
        }
        "LOCK" => {
            let parsed = parse_json_body(&body);
            let Ok(lock_info) = parsed else {
                return bad_request("lock payload must be valid JSON");
            };

            match app_state
                .store
                .acquire_lock(&resource_key, &lock_info)
                .await
            {
                Ok(AcquireLockResult::Acquired) => StatusCode::OK.into_response(),
                Ok(AcquireLockResult::AlreadyLocked(existing)) => {
                    (StatusCode::LOCKED, Json(existing)).into_response()
                }
                Err(_) => internal_error_response(),
            }
        }
        "UNLOCK" => {
            let expected_lock_id = if body.is_empty() {
                None
            } else {
                let parsed = parse_json_body(&body);
                let Ok(lock_info) = parsed else {
                    return bad_request("unlock payload must be valid JSON");
                };
                lock_info
                    .get("ID")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            };

            match app_state
                .store
                .release_lock(&resource_key, expected_lock_id.as_deref())
                .await
            {
                Ok(ReleaseLockResult::Released) | Ok(ReleaseLockResult::NotLocked) => {
                    StatusCode::OK.into_response()
                }
                Ok(ReleaseLockResult::LockMismatch(existing)) => {
                    (StatusCode::CONFLICT, Json(existing)).into_response()
                }
                Err(_) => internal_error_response(),
            }
        }
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

async fn ensure_lock_allows_write(
    app_state: &AppState,
    resource_key: &str,
    lock_id: Option<&str>,
) -> Option<Response> {
    let current_lock = match app_state.store.get_lock(resource_key).await {
        Ok(current_lock) => current_lock,
        Err(_) => return Some(internal_error_response()),
    };
    let current_lock = current_lock?;
    let current_id = current_lock
        .get("ID")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if Some(current_id) == lock_id {
        return None;
    }

    Some((StatusCode::LOCKED, Json(current_lock)).into_response())
}

fn parse_json_body(body: &Bytes) -> Result<Value, serde_json::Error> {
    if body.is_empty() {
        Ok(json!({}))
    } else {
        serde_json::from_slice(body)
    }
}

fn unauthorized_response() -> Response {
    let mut response = StatusCode::UNAUTHORIZED.into_response();
    response.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        header::HeaderValue::from_static("Basic realm=\"tf-http-backend\""),
    );
    response
}

fn bad_request(msg: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
}

fn internal_error_response() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal server error" })),
    )
        .into_response()
}

fn is_authorized(headers: &HeaderMap, auth: &Option<BasicAuth>) -> bool {
    let Some(auth) = auth else {
        return true;
    };

    let Some(value) = headers.get(header::AUTHORIZATION) else {
        return false;
    };

    let Ok(value) = value.to_str() else {
        return false;
    };

    let Some(encoded) = value.strip_prefix("Basic ") else {
        return false;
    };

    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
        return false;
    };

    let Ok(decoded) = String::from_utf8(bytes) else {
        return false;
    };

    let Some((username, password)) = decoded.split_once(':') else {
        return false;
    };

    username == auth.username && password == auth.password
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::{to_bytes, Body},
        http::{Method, Request, StatusCode},
    };
    use serde_json::json;
    use tower::ServiceExt;

    use crate::{
        app::{build_router, AppState},
        store::{AcquireLockResult, InMemoryStore, ReleaseLockResult, StateStore, StoreError},
    };

    #[tokio::test]
    async fn get_missing_state_returns_404() {
        let app = build_router(AppState {
            store: Arc::new(InMemoryStore::new()),
            basic_auth: None,
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/state/demo")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn lock_then_write_without_id_returns_423() {
        let app = build_router(AppState {
            store: Arc::new(InMemoryStore::new()),
            basic_auth: None,
        });

        let lock_request = Request::builder()
            .uri("/state/demo")
            .method(Method::from_bytes(b"LOCK").unwrap())
            .header("content-type", "application/json")
            .body(Body::from(json!({"ID":"lock-1"}).to_string()))
            .unwrap();
        let lock_response = app.clone().oneshot(lock_request).await.unwrap();
        assert_eq!(lock_response.status(), StatusCode::OK);

        let write_request = Request::builder()
            .uri("/state/demo")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(json!({"version":4}).to_string()))
            .unwrap();
        let write_response = app.oneshot(write_request).await.unwrap();
        assert_eq!(write_response.status(), StatusCode::LOCKED);
    }

    #[tokio::test]
    async fn lock_then_write_with_id_succeeds() {
        let app = build_router(AppState {
            store: Arc::new(InMemoryStore::new()),
            basic_auth: None,
        });

        let lock_request = Request::builder()
            .uri("/state/demo")
            .method(Method::from_bytes(b"LOCK").unwrap())
            .header("content-type", "application/json")
            .body(Body::from(json!({"ID":"lock-2"}).to_string()))
            .unwrap();
        let lock_response = app.clone().oneshot(lock_request).await.unwrap();
        assert_eq!(lock_response.status(), StatusCode::OK);

        let write_request = Request::builder()
            .uri("/state/demo?ID=lock-2")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(json!({"version":4,"serial":1}).to_string()))
            .unwrap();
        let write_response = app.clone().oneshot(write_request).await.unwrap();
        assert_eq!(write_response.status(), StatusCode::OK);

        let read_request = Request::builder()
            .uri("/state/demo")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let read_response = app.oneshot(read_request).await.unwrap();
        assert_eq!(read_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn state_round_trip_then_delete_returns_404() {
        let app = build_router(AppState {
            store: Arc::new(InMemoryStore::new()),
            basic_auth: None,
        });

        let write_request = Request::builder()
            .uri("/state/smoke")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(json!({"version":4,"serial":7}).to_string()))
            .unwrap();
        let write_response = app.clone().oneshot(write_request).await.unwrap();
        assert_eq!(write_response.status(), StatusCode::OK);

        let read_request = Request::builder()
            .uri("/state/smoke")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let read_response = app.clone().oneshot(read_request).await.unwrap();
        assert_eq!(read_response.status(), StatusCode::OK);
        let read_body = to_bytes(read_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let read_json: serde_json::Value = serde_json::from_slice(&read_body).unwrap();
        assert_eq!(read_json, json!({"version": 4, "serial": 7}));

        let delete_request = Request::builder()
            .uri("/state/smoke")
            .method("DELETE")
            .body(Body::empty())
            .unwrap();
        let delete_response = app.clone().oneshot(delete_request).await.unwrap();
        assert_eq!(delete_response.status(), StatusCode::OK);

        let read_after_delete_request = Request::builder()
            .uri("/state/smoke")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let read_after_delete_response = app.oneshot(read_after_delete_request).await.unwrap();
        assert_eq!(read_after_delete_response.status(), StatusCode::NOT_FOUND);
    }

    struct GetLockErrorStore {
        upserted: std::sync::atomic::AtomicBool,
    }

    #[async_trait::async_trait]
    impl StateStore for GetLockErrorStore {
        async fn get_state(&self, _key: &str) -> Result<Option<serde_json::Value>, StoreError> {
            Ok(None)
        }

        async fn upsert_state(
            &self,
            _key: &str,
            _state: &serde_json::Value,
        ) -> Result<(), StoreError> {
            self.upserted
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        async fn delete_state(&self, _key: &str) -> Result<(), StoreError> {
            Ok(())
        }

        async fn get_lock(&self, _key: &str) -> Result<Option<serde_json::Value>, StoreError> {
            Err(StoreError::Db(sqlx::Error::Protocol(
                "injected get_lock failure".into(),
            )))
        }

        async fn acquire_lock(
            &self,
            _key: &str,
            _lock_info: &serde_json::Value,
        ) -> Result<AcquireLockResult, StoreError> {
            Ok(AcquireLockResult::Acquired)
        }

        async fn release_lock(
            &self,
            _key: &str,
            _expected_lock_id: Option<&str>,
        ) -> Result<ReleaseLockResult, StoreError> {
            Ok(ReleaseLockResult::NotLocked)
        }
    }

    #[tokio::test]
    async fn write_returns_500_when_lock_lookup_fails() {
        let store = Arc::new(GetLockErrorStore {
            upserted: std::sync::atomic::AtomicBool::new(false),
        });
        let app = build_router(AppState {
            store: store.clone(),
            basic_auth: None,
        });

        let write_request = Request::builder()
            .uri("/state/demo")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(json!({"version":4}).to_string()))
            .unwrap();
        let write_response = app.oneshot(write_request).await.unwrap();
        assert_eq!(write_response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!store.upserted.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn unlock_with_matching_id_releases_lock() {
        let app = build_router(AppState {
            store: Arc::new(InMemoryStore::new()),
            basic_auth: None,
        });

        let lock_request = Request::builder()
            .uri("/state/demo")
            .method(Method::from_bytes(b"LOCK").unwrap())
            .header("content-type", "application/json")
            .body(Body::from(json!({"ID":"lock-3"}).to_string()))
            .unwrap();
        let lock_response = app.clone().oneshot(lock_request).await.unwrap();
        assert_eq!(lock_response.status(), StatusCode::OK);

        let unlock_request = Request::builder()
            .uri("/state/demo")
            .method(Method::from_bytes(b"UNLOCK").unwrap())
            .header("content-type", "application/json")
            .body(Body::from(json!({"ID":"lock-3"}).to_string()))
            .unwrap();
        let unlock_response = app.clone().oneshot(unlock_request).await.unwrap();
        assert_eq!(unlock_response.status(), StatusCode::OK);

        let write_request = Request::builder()
            .uri("/state/demo")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(json!({"version":4}).to_string()))
            .unwrap();
        let write_response = app.oneshot(write_request).await.unwrap();
        assert_eq!(write_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn unlock_with_mismatched_id_returns_409() {
        let app = build_router(AppState {
            store: Arc::new(InMemoryStore::new()),
            basic_auth: None,
        });

        let lock_request = Request::builder()
            .uri("/state/demo")
            .method(Method::from_bytes(b"LOCK").unwrap())
            .header("content-type", "application/json")
            .body(Body::from(json!({"ID":"lock-4"}).to_string()))
            .unwrap();
        let lock_response = app.clone().oneshot(lock_request).await.unwrap();
        assert_eq!(lock_response.status(), StatusCode::OK);

        let unlock_request = Request::builder()
            .uri("/state/demo")
            .method(Method::from_bytes(b"UNLOCK").unwrap())
            .header("content-type", "application/json")
            .body(Body::from(json!({"ID":"other"}).to_string()))
            .unwrap();
        let unlock_response = app.oneshot(unlock_request).await.unwrap();
        assert_eq!(unlock_response.status(), StatusCode::CONFLICT);
        let unlock_body = to_bytes(unlock_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let unlock_json: serde_json::Value = serde_json::from_slice(&unlock_body).unwrap();
        assert_eq!(unlock_json, json!({"ID":"lock-4"}));
    }
}
