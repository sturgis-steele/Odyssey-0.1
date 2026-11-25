use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
};
use odyssey_backend::{application, config::Config};
use reqwest::Client;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, VecDeque},
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Duration,
};
use tempfile::TempDir;
use tokio::{
    net::TcpListener,
    task::JoinHandle,
    time::{sleep, timeout},
};
use tokio_stream::wrappers::ReceiverStream;

fn packet(delta: Value, finish: Option<&str>) -> String {
    format!(
        "data: {}\r\n\r\n",
        json!({"choices":[{"delta":delta,"finish_reason":finish}]})
    )
}
fn answer(text: &str) -> String {
    packet(json!({"content":text}), None) + &packet(json!({}), Some("stop")) + "data: [DONE]\n\n"
}
#[derive(Clone)]
struct Mock {
    rounds: Arc<Mutex<VecDeque<String>>>,
    requests: Arc<Mutex<Vec<Value>>>,
    mode: Arc<Mutex<String>>,
}
async fn upstream(State(m): State<Mock>, uri: Uri, body: Option<Json<Value>>) -> Response {
    let mode = m.mode.lock().unwrap().clone();
    let b = body.map(|b| b.0).unwrap_or_default();
    match uri.path(){
        "/health" if mode=="loading"=>(StatusCode::SERVICE_UNAVAILABLE,"loading").into_response(),
        "/health"=>Json(json!({"status":"ok"})).into_response(),
        "/v1/models"=>Json(json!({"data":[{"id":"file.gguf","aliases":[if mode=="alias"{"other"}else{"odyssey-local"}]}]})).into_response(),
        "/props"=>Json(json!({"chat_template_caps":{"supports_tools":mode!="no_tools"},"default_generation_settings":{"n_ctx":4096}})).into_response(),
        "/apply-template"=>{
            assert_eq!(b["tools"].as_array().unwrap().len(),3);
            assert_eq!(b["chat_template_kwargs"]["enable_thinking"],false);
            Json(json!({"prompt":b["messages"].to_string()})).into_response()
        }
        "/tokenize"=>{
            assert_eq!(b["parse_special"],true);
            let len=serde_json::from_str::<Vec<Value>>(b["content"].as_str().unwrap()).unwrap().len();
            let n=if mode=="full" || (mode=="prune" && len>2){4000}else{40};
            Json(json!({"tokens":vec![1;n]})).into_response()
        }
        "/v1/chat/completions"=>{
            assert_eq!(b["stream"],true);
            assert_eq!(b["model"],"odyssey-local");
            m.requests.lock().unwrap().push(b);
            let data=m.rounds.lock().unwrap().pop_front().expect("fixture round");
            if data=="@wait" {
                let (tx,recv)=tokio::sync::mpsc::channel(1);
                tokio::spawn(async move{
                    let _=tx.send(Ok::<_,Infallible>(packet(json!({"content":"partial"}),None))).await;
                    tx.closed().await;
                });
                Body::from_stream(ReceiverStream::new(recv)).into_response()
            }else{data.into_response()}
        }
        _=>StatusCode::NOT_FOUND.into_response()
    }
}
struct Fixture {
    _dir: TempDir,
    api: String,
    client: Client,
    model: Mock,
    model_task: JoinHandle<()>,
    backend: JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.model_task.abort();
        self.backend.abort();
    }
}
impl Fixture {
    async fn new(rounds: Vec<String>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let model = Mock {
            rounds: Arc::new(Mutex::new(rounds.into())),
            requests: Default::default(),
            mode: Default::default(),
        };
        let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/v1", socket.local_addr().unwrap());
        let mock = Router::new()
            .fallback(get(upstream).post(upstream))
            .with_state(model.clone());
        let model_task = tokio::spawn(async move {
            axum::serve(socket, mock).await.unwrap();
        });
        let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();
        let vars = HashMap::from([
            ("BACKEND_PORT".into(), addr.port().to_string()),
            ("LLAMA_BASE_URL".into(), url),
            ("MODEL_TIMEOUT_SECONDS".into(), "1".into()),
            ("MAX_TOOL_ITERATIONS".into(), "2".into()),
        ]);
        let cfg = Config::from_values(dir.path(), &vars).unwrap();
        let app = application(cfg).await.unwrap();
        let backend = tokio::spawn(async move {
            axum::serve(socket, app).await.unwrap();
        });
        Self {
            _dir: dir,
            api: format!("http://{addr}/api"),
            client: Client::builder().no_proxy().build().unwrap(),
            model,
            model_task,
            backend,
        }
    }
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.api)
    }
    async fn get(&self, path: &str) -> Value {
        self.client
            .get(self.url(path))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }
    async fn create(&self) -> String {
        let r = self
            .client
            .post(self.url("/conversations"))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 201);
        r.json::<Value>().await.unwrap()["id"]
            .as_str()
            .unwrap()
            .into()
    }
    async fn open_stream(&self, id: &str, msg: &str) -> reqwest::Response {
        self.client
            .post(self.url("/chat/stream"))
            .json(&json!({"conversation_id":id,"message":msg}))
            .send()
            .await
            .unwrap()
    }
    async fn events(&self, id: &str, msg: &str) -> Vec<(String, Value)> {
        let r = self.open_stream(id, msg).await;
        assert_eq!(r.status(), 200);
        assert!(
            r.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("text/event-stream")
        );
        let body = r.text().await.unwrap();
        body.trim_end()
            .split("\n\n")
            .map(|frame| {
                let mut lines = frame.lines();
                (
                    lines
                        .next()
                        .unwrap()
                        .strip_prefix("event: ")
                        .unwrap()
                        .into(),
                    serde_json::from_str(lines.next().unwrap().strip_prefix("data: ").unwrap())
                        .unwrap(),
                )
            })
            .collect()
    }
    fn mode(&self, name: &str) {
        *self.model.mode.lock().unwrap() = name.into();
    }
}
#[tokio::test]
async fn http_contract_health_validation_tools_and_origins() {
    let f = Fixture::new(vec![]).await;
    assert_eq!(
        f.get("/health").await,
        json!({"backend":"online","version":"0.1.0"})
    );
    for (mode, state, ready) in [
        ("", "online", true),
        ("loading", "loading", false),
        ("alias", "misconfigured", false),
    ] {
        f.mode(mode);
        let s = f.get("/system/status").await;
        assert_eq!(s["database"], "online");
        assert_eq!(s["model_server"], state);
        assert_eq!(s["model_ready"], ready);
        assert_eq!(s["model_reachable"], true);
        assert_eq!(f.get("/models/current").await["model_server"], state);
    }
    assert_eq!(f.get("/tools").await.as_array().unwrap().len(), 3);
    let run = |name: &str, args: Value| {
        f.client
            .post(f.url(&format!("/tools/{name}/run")))
            .json(&args)
            .send()
    };
    assert_eq!(
        run(
            "calculator",
            json!({"arguments":{"expression":"157 * 243"}})
        )
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap()["result"]["value"],
        38151
    );
    assert_eq!(run("unknown", json!({})).await.unwrap().status(), 404);
    assert_eq!(
        run("calculator", json!({"arguments":null}))
            .await
            .unwrap()
            .status(),
        422
    );
    assert_eq!(
        run("calculator", json!({"wrong":true}))
            .await
            .unwrap()
            .status(),
        422
    );
    for (url, status) in [
        ("/conversations?limit=0", 422),
        ("/conversations?offset=-1", 422),
        ("/conversations/not-a-uuid", 422),
        ("/absent", 404),
    ] {
        assert_eq!(
            f.client.get(f.url(url)).send().await.unwrap().status(),
            status
        );
    }
    let denied = f
        .client
        .post(f.url("/conversations"))
        .header("Origin", "https://outside.example")
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), 403);
    assert_eq!(
        f.client
            .get(f.url("/health"))
            .header("host", "unknown.example")
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    let with_origin = f
        .client
        .get(f.url("/health"))
        .header("Origin", "http://localhost:5173")
        .send()
        .await
        .unwrap();
    assert_eq!(
        with_origin.headers()["access-control-allow-origin"],
        "http://localhost:5173"
    );
    assert_eq!(with_origin.headers()["cache-control"], "no-store");
    let r = f.open_stream("bad", "sensitive-input").await;
    assert_eq!(r.status(), 422);
    assert!(!r.text().await.unwrap().contains("sensitive-input"));
    f.model_task.abort();
    let s = f.get("/system/status").await;
    assert_eq!(s["backend"], "online");
    assert_eq!(s["model_reachable"], false);
    assert_eq!(s["status"], "degraded");
    let cid = f.create().await;
    assert_eq!(
        f.events(&cid, "hello").await.last().unwrap().1["code"],
        "model_offline"
    );
}
#[tokio::test]
async fn fragmented_multi_tool_rounds_and_history() {
    let a = packet(
        json!({"reasoning_content":"private thought","tool_calls":[{"index":1,"id":"two","function":{"name":"calculator","arguments":"broken"}},{"index":0,"id":"one","function":{"name":"calcu","arguments":"{\"ex"}}]}),
        None,
    ) + &packet(
        json!({"tool_calls":[{"index":0,"function":{"name":"lator","arguments":"pression\":\"6*7\"}"}}]}),
        Some("tool_calls"),
    );
    let f = Fixture::new(vec![a, answer("Forty two."), answer("Again.")]).await;
    let id = f.create().await;
    let e = f.events(&id, "  multiply  ").await;
    assert_eq!(
        e.iter().map(|v| v.0.as_str()).collect::<Vec<_>>(),
        [
            "message_start",
            "tool_start",
            "tool_result",
            "tool_start",
            "tool_result",
            "text_delta",
            "message_complete"
        ]
    );
    assert_eq!(e[2].1["call_id"], "one");
    assert_eq!(e[2].1["result"]["value"], 42);
    assert_eq!(e[4].1["ok"], false);
    let req = f.model.requests.lock().unwrap()[1]["messages"].clone();
    assert_eq!(req[2]["reasoning_content"], "private thought");
    assert_eq!(req[2]["tool_calls"][1]["function"]["arguments"], "{}");
    assert_eq!(req[3]["tool_call_id"], "one");
    assert_eq!(req[4]["tool_call_id"], "two");
    assert_eq!(
        f.events(&id, "again").await.last().unwrap().0,
        "message_complete"
    );
    let req = f.model.requests.lock().unwrap()[2]["messages"].clone();
    assert!(!req.to_string().contains("private thought"));
    assert_eq!(req.as_array().unwrap().len(), 7);
    let detail = f.get(&format!("/conversations/{id}?limit=1")).await;
    let page = f
        .get(&format!(
            "/conversations/{id}?before={}",
            detail["next_before"]
        ))
        .await;
    assert_eq!(page["turns"][0]["user"], "multiply");
    assert_eq!(page["turns"][0]["tools"].as_array().unwrap().len(), 2);
    assert_eq!(f.get("/conversations").await[0]["title"], "multiply");
    assert_eq!(
        f.client
            .delete(f.url(&format!("/conversations/{id}")))
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    assert_eq!(
        f.get(&format!("/conversations/{id}")).await["code"],
        "not_found"
    );
}
#[tokio::test]
async fn error_terminals_limits_and_context() {
    let unsupported = packet(
        json!({"tool_calls":[{"function":{"name":"unavailable","arguments":{}}}]}),
        Some("tool_calls"),
    );
    let f = Fixture::new(vec![
        unsupported.clone(),
        unsupported.clone(),
        unsupported,
        packet(json!({"content":"unfinished"}), None),
        packet(json!({}), Some("length")),
        "data: invalid\n\n".into(),
        answer("first"),
        answer("next"),
    ])
    .await;
    let id = f.create().await;
    let events = f.events(&id, "tools").await;
    assert_eq!(
        events
            .iter()
            .filter(|(k, v)| k == "tool_result" && v["ok"] == false)
            .count(),
        2
    );
    assert_eq!(events.last().unwrap().1["code"], "tool_limit");
    for code in ["model_protocol", "output_limit", "model_protocol"] {
        assert_eq!(f.events(&id, "test").await.last().unwrap().1["code"], code);
    }
    let turns = f.get(&format!("/conversations/{id}")).await;
    assert_eq!(turns["turns"][1]["status"], "error");
    assert_eq!(turns["turns"][1]["assistant"], "unfinished");
    f.events(&id, "first").await;
    f.mode("prune");
    f.events(&id, "again").await;
    assert_eq!(
        f.model.requests.lock().unwrap().last().unwrap()["messages"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for (mode, code) in [("full", "context_limit"), ("no_tools", "tool_template")] {
        f.mode(mode);
        assert_eq!(f.events(&id, "test").await.last().unwrap().1["code"], code);
    }
}
#[tokio::test]
async fn progressive_text_timeout_disconnect_and_slot_reuse() {
    let f = Fixture::new(vec!["@wait".into(), "@wait".into(), answer("recovered")]).await;
    let id = f.create().await;
    let mut r = f.open_stream(&id, "cancel").await;
    let mut body = String::new();
    timeout(Duration::from_millis(900), async {
        while !body.contains("partial") {
            body.push_str(std::str::from_utf8(&r.chunk().await.unwrap().unwrap()).unwrap());
        }
    })
    .await
    .expect("partial text arrives before upstream finishes");
    assert_eq!(f.open_stream(&id, "overlap").await.status(), 409);
    assert_eq!(
        f.client
            .delete(f.url(&format!("/conversations/{id}")))
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    drop(r);
    timeout(Duration::from_secs(3), async {
        loop {
            let turn = &f.get(&format!("/conversations/{id}")).await["turns"][0];
            if turn["status"] == "interrupted" {
                assert_eq!(turn["assistant"], "partial");
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        f.events(&id, "timeout").await.last().unwrap().1["code"],
        "model_timeout"
    );
    assert_eq!(
        f.events(&id, "retry").await.last().unwrap().0,
        "message_complete"
    );
}

#[tokio::test]
async fn locked_final_snapshot_closes_error_stream_and_recovers() {
    let f = Fixture::new(vec!["@wait".into(), answer("after lock")]).await;
    let id = f.create().await;
    let mut stream = f.open_stream(&id, "locked finalization").await;
    let mut prefix = String::new();
    while !prefix.contains("partial") {
        prefix.push_str(std::str::from_utf8(&stream.chunk().await.unwrap().unwrap()).unwrap());
    }
    let db = rusqlite::Connection::open(f._dir.path().join("data/odyssey.db")).unwrap();
    db.execute_batch("BEGIN IMMEDIATE").unwrap();
    // An open client gets a terminal storage error even while the worker still owns cleanup.
    let rest = timeout(Duration::from_secs(5), stream.text())
        .await
        .unwrap()
        .unwrap();
    assert!(rest.contains("event: error") && rest.contains("\"code\":\"storage\""));
    assert_eq!(f.open_stream(&id, "overlap").await.status(), 409);
    db.execute_batch("ROLLBACK").unwrap();
    timeout(Duration::from_secs(3), async {
        loop {
            if f.get(&format!("/conversations/{id}")).await["turns"][0]["status"] == "error" {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        f.events(&id, "retry").await.last().unwrap().0,
        "message_complete"
    );
}
