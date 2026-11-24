use crate::{
    VERSION,
    config::{Config, local_host},
    database::Store,
    error::{Error, Result},
    provider::{Llama, ModelProvider},
    tools::Registry,
    turn,
};
use axum::{
    Json, Router,
    body::to_bytes,
    extract::{FromRequest, Path, Query, Request, State, rejection::QueryRejection},
    http::{HeaderValue, Method, StatusCode, header, uri::Authority},
    middleware::{self, Next},
    response::{IntoResponse, Response, Sse},
    routing::{get, post},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Map, Value, json};
use std::{convert::Infallible, sync::Arc};
use tokio::sync::Semaphore;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
use tower_http::{cors::CorsLayer, services::ServeDir};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct StateData {
    pub cfg: Arc<Config>,
    pub store: Store,
    pub model: Llama,
    pub tools: Registry,
    slot: Arc<Semaphore>,
}
struct Body<T>(T);
impl<S: Sync, T: DeserializeOwned> FromRequest<S> for Body<T> {
    type Rejection = Error;
    async fn from_request(req: Request, _: &S) -> Result<Self> {
        if let Some(kind) = req.headers().get(header::CONTENT_TYPE) {
            let media = kind
                .to_str()
                .unwrap_or("")
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            if !(media == "application/json"
                || (media.starts_with("application/") && media.ends_with("+json")))
            {
                return Err(Error::invalid());
            }
        }
        let raw = to_bytes(req.into_body(), 256 * 1024)
            .await
            .map_err(|_| Error::invalid())?;
        serde_json::from_slice(&raw)
            .map(Body)
            .map_err(|_| Error::invalid())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Chat {
    conversation_id: Uuid,
    message: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manual {
    #[serde(default)]
    arguments: Map<String, Value>,
}
#[derive(Deserialize)]
struct ListQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}
#[derive(Deserialize)]
struct PageQuery {
    limit: Option<i64>,
    before: Option<i64>,
}
fn limit(v: Option<i64>, default: i64) -> Result<i64> {
    let n = v.unwrap_or(default);
    if (1..=100).contains(&n) {
        Ok(n)
    } else {
        Err(Error::invalid())
    }
}
fn id(cid: &str) -> Result<String> {
    Uuid::parse_str(cid)
        .map(|u| u.to_string())
        .map_err(|_| Error::invalid())
}

async fn health() -> Json<Value> {
    Json(json!({"backend":"online","version":VERSION}))
}
async fn current(State(s): State<StateData>) -> Json<Value> {
    Json(s.model.health().await.public())
}
async fn status(State(s): State<StateData>) -> Json<Value> {
    let (model, db) = tokio::join!(s.model.health(), s.store.ping());
    let ready = model.state == "online";
    Json(json!({
        "backend": "online", "model": model.model, "model_server": model.state, "diagnostic": model.diagnostic,
        "status": if ready && db.is_ok() {"ready"} else {"degraded"},
        "database": if db.is_ok() {"online"} else {"offline"},
        "model_reachable": model.reachable, "model_ready": ready,
        "version": VERSION, "context_tokens": s.cfg.context, "max_output_tokens": s.cfg.max_output,
    }))
}
async fn catalog(State(s): State<StateData>) -> Json<Vec<Value>> {
    Json(s.tools.catalog())
}
async fn manual(
    State(s): State<StateData>,
    Path(name): Path<String>,
    Body(run): Body<Manual>,
) -> Result<Json<Value>> {
    if !s.tools.contains(&name) {
        return Err(Error::new(
            "unknown_tool",
            "This tool is unavailable.",
            StatusCode::NOT_FOUND,
        ));
    }
    let outcome = tokio::task::spawn_blocking(move || {
        s.tools.run(&name, Some(&Value::Object(run.arguments)))
    })
    .await
    .map_err(|_| {
        Error::new(
            "internal",
            "The device could not complete this tool.",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    Ok(Json(outcome))
}
async fn create(State(s): State<StateData>) -> Result<impl IntoResponse> {
    Ok((StatusCode::CREATED, Json(s.store.create().await?)))
}
async fn list(
    State(s): State<StateData>,
    params: std::result::Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<Vec<Value>>> {
    let Query(q) = params.map_err(|_| Error::invalid())?;
    let offset = q.offset.unwrap_or(0);
    if offset < 0 {
        return Err(Error::invalid());
    }
    Ok(Json(s.store.list(limit(q.limit, 30)?, offset).await?))
}
async fn detail(
    State(s): State<StateData>,
    Path(cid): Path<String>,
    params: std::result::Result<Query<PageQuery>, QueryRejection>,
) -> Result<Json<Value>> {
    let Query(q) = params.map_err(|_| Error::invalid())?;
    let before = q.before.unwrap_or(i64::MAX);
    if before < 1 {
        return Err(Error::invalid());
    }
    Ok(Json(
        s.store
            .detail(id(&cid)?, limit(q.limit, 20)?, before)
            .await?,
    ))
}
async fn remove(State(s): State<StateData>, Path(cid): Path<String>) -> Result<StatusCode> {
    s.store.delete(id(&cid)?).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn chat(State(s): State<StateData>, Body(body): Body<Chat>) -> Result<Response> {
    let message = body.message.trim();
    if !(1..=8192).contains(&message.chars().count()) {
        return Err(Error::invalid());
    }
    let permit = s
        .slot
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::busy())?;
    let events = turn::start(s, permit, body.conversation_id.to_string(), message.into()).await?;
    let mut response =
        Sse::new(ReceiverStream::new(events).map(Ok::<_, Infallible>)).into_response();
    response
        .headers_mut()
        .insert("x-accel-buffering", HeaderValue::from_static("no"));
    Ok(response)
}
async fn openapi(State(s): State<StateData>) -> Json<Value> {
    let mut schema: Value =
        serde_json::from_str(include_str!("../openapi.json")).expect("bundled OpenAPI");
    schema["info"]["title"] = json!(s.cfg.app_name);
    Json(schema)
}
fn problem(status: StatusCode, detail: &str) -> Response {
    (status, Json(json!({"detail":detail}))).into_response()
}
async fn missing() -> Response {
    problem(StatusCode::NOT_FOUND, "Not Found")
}
fn known_route(path: &str) -> bool {
    match path
        .trim_start_matches('/')
        .split('/')
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["api", "health"]
        | ["api", "models", "current"]
        | ["api", "system", "status"]
        | ["api", "tools"]
        | ["api", "chat", "stream"]
        | ["api", "conversations"] => true,
        ["api", "tools", name, "run"] | ["api", "conversations", name] => !name.is_empty(),
        _ => false,
    }
}
async fn local(State(origins): State<Arc<Vec<HeaderValue>>>, req: Request, next: Next) -> Response {
    let path = req.uri().path();
    let api = path.starts_with("/api/");
    let headers = req.headers();
    let host = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.parse::<Authority>().ok());
    let bad_origin = headers
        .get_all(header::ORIGIN)
        .iter()
        .any(|v| !origins.contains(v))
        || headers.get_all(header::ORIGIN).iter().count() > 1;
    let mut response = if bad_origin {
        problem(StatusCode::FORBIDDEN, "This origin cannot access Odyssey.")
    } else if headers.get_all(header::HOST).iter().count() != 1
        || !host.is_some_and(|h| {
            local_host(h.host().trim_start_matches('[').trim_end_matches(']'))
                && !h.as_str().contains('@')
        })
    {
        (StatusCode::BAD_REQUEST, "Invalid host header").into_response()
    } else if path.ends_with('/') && known_route(path.trim_end_matches('/')) {
        let query = req
            .uri()
            .query()
            .map(|q| format!("?{q}"))
            .unwrap_or_default();
        axum::response::Redirect::temporary(&format!("{}{query}", path.trim_end_matches('/')))
            .into_response()
    } else if req.method() == Method::HEAD && known_route(path) {
        problem(StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed")
    } else {
        next.run(req).await
    };
    let h = response.headers_mut();
    if api {
        h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    h.insert(header::CONTENT_SECURITY_POLICY,HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self'; font-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'"));
    response
}
pub async fn application(cfg: Config) -> std::result::Result<Router, String> {
    let dist = cfg.root.join("frontend/dist");
    let mut origins = Vec::new();
    for host in ["127.0.0.1", "localhost", "[::1]"] {
        for port in [5173, cfg.port] {
            let origin =
                HeaderValue::from_str(&format!("http://{host}:{port}")).expect("local origin");
            if !origins.contains(&origin) {
                origins.push(origin);
            }
        }
    }
    let cors = CorsLayer::new()
        .allow_origin(origins.clone())
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE]);
    let cfg = Arc::new(cfg);
    let state = StateData {
        model: Llama::new(cfg.clone()).map_err(|e| e.to_string())?,
        store: Store::open(cfg.database.clone()).await?,
        tools: Registry::new(cfg.timezone),
        cfg,
        slot: Arc::new(Semaphore::new(1)),
    };
    let api = Router::new()
        .route("/health", get(health))
        .route("/models/current", get(current))
        .route("/system/status", get(status))
        .route("/tools", get(catalog))
        .route("/tools/{name}/run", post(manual))
        .route("/conversations", get(list).post(create))
        .route("/conversations/{cid}", get(detail).delete(remove))
        .route("/chat/stream", post(chat))
        .method_not_allowed_fallback(|| async {
            problem(StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed")
        })
        .fallback(missing);
    let mut app = Router::new()
        .nest("/api", api)
        .route("/openapi.json", get(openapi));
    if dist.join("index.html").is_file() {
        app = app.fallback_service(ServeDir::new(dist));
    } else {
        app = app.fallback(missing);
    }
    Ok(app
        .with_state(state)
        .layer(cors)
        .layer(middleware::from_fn_with_state(Arc::new(origins), local)))
}
