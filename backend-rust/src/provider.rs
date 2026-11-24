use crate::{
    config::Config,
    error::{Error, Result},
};
use axum::http::StatusCode;
use reqwest::{Client, RequestBuilder, Response, redirect::Policy};
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, HashSet},
    future::Future,
    sync::Arc,
    time::Duration,
};
use uuid::Uuid;

pub struct Health {
    pub model: String,
    pub state: &'static str,
    pub diagnostic: &'static str,
    pub reachable: bool,
}
impl Health {
    pub fn public(&self) -> Value {
        json!({"model":self.model,"model_server":self.state,"diagnostic":self.diagnostic})
    }
}
#[derive(Default)]
pub struct Call {
    pub id: String,
    pub name: String,
    pub arguments: String,
}
impl Call {
    pub fn parsed(&self) -> Option<Value> {
        serde_json::from_str::<Value>(&self.arguments)
            .ok()
            .filter(Value::is_object)
    }
    pub fn wire(&self) -> Value {
        let arguments = if self.parsed().is_some() {
            &self.arguments
        } else {
            "{}"
        };
        json!({"id":self.id,"type":"function","function":{"name":self.name,"arguments":arguments}})
    }
}
pub enum Part {
    Text(String),
    Finished { calls: Vec<Call>, reasoning: String },
}
pub trait ModelStream: Send {
    fn next(&mut self) -> impl Future<Output = Result<Part>> + Send;
}
pub trait ModelProvider: Sync {
    type Stream: ModelStream;
    fn health(&self) -> impl Future<Output = Health> + Send;
    fn capacity(&self) -> impl Future<Output = Result<usize>> + Send;
    fn prompt_tokens(
        &self,
        messages: &[Value],
        tools: &[Value],
    ) -> impl Future<Output = Result<usize>> + Send;
    fn stream(
        &self,
        messages: &[Value],
        tools: &[Value],
    ) -> impl Future<Output = Result<Self::Stream>> + Send;
    fn complete(
        &self,
        messages: &[Value],
        tools: &[Value],
    ) -> impl Future<Output = Result<(String, Vec<Call>)>> + Send;
}
#[derive(Clone)]
pub struct Llama {
    cfg: Arc<Config>,
    root: String,
    client: Client,
}
fn transport(e: reqwest::Error) -> Error {
    if e.is_timeout() {
        Error::new(
            "model_timeout",
            "The local model timed out. Try a shorter request.",
            StatusCode::GATEWAY_TIMEOUT,
        )
    } else {
        Error::new(
            "model_offline",
            "The local model connection was lost. Check llama-server and try again.",
            StatusCode::SERVICE_UNAVAILABLE,
        )
    }
}
async fn bounded_body(mut response: Response, max: usize) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport)? {
        if body.len() + chunk.len() > max {
            return Err(Error::protocol());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
async fn checked(request: RequestBuilder) -> Result<Response> {
    let r = request.send().await.map_err(transport)?;
    let status = r.status();
    if status.is_success() {
        return Ok(r);
    }
    let body = bounded_body(r, 65536).await?;
    if serde_json::from_slice::<Value>(&body)
        .ok()
        .and_then(|v| v.get("error").cloned())
        .is_some_and(|e| e.to_string().to_lowercase().contains("context"))
    {
        return Err(Error::bad_turn(
            "context_limit",
            "The model context is full. Shorten the message or start a new conversation.",
        ));
    }
    if status == StatusCode::SERVICE_UNAVAILABLE {
        return Err(Error::new(
            "model_loading",
            "The local model is loading or temporarily unavailable.",
            status,
        ));
    }
    Err(Error::new(
        "model_request",
        "llama-server rejected the request. Check the model alias, server version and tool template.",
        StatusCode::BAD_GATEWAY,
    ))
}
async fn document(req: RequestBuilder) -> Result<Value> {
    serde_json::from_slice(&bounded_body(checked(req).await?, 8 * 1024 * 1024).await?)
        .map_err(|_| Error::protocol())
}
impl Llama {
    pub fn new(cfg: Arc<Config>) -> Result<Self> {
        let client = Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(3))
            .read_timeout(cfg.timeout)
            .build()
            .map_err(transport)?;
        Ok(Self {
            root: cfg.base_url.trim_end_matches("/v1").to_string(),
            cfg,
            client,
        })
    }
    fn get(&self, path: &str) -> RequestBuilder {
        self.client.get(format!("{}{path}", self.root))
    }
    fn post(&self, path: &str, data: &Value) -> RequestBuilder {
        self.client.post(format!("{}{path}", self.root)).json(data)
    }
    fn payload(&self, messages: &[Value], tools: &[Value]) -> Value {
        json!({"model":self.cfg.model,"messages":messages,"tools":tools,"tool_choice":"auto","parallel_tool_calls":false,
            "chat_template_kwargs":{"enable_thinking":self.cfg.thinking}})
    }
}
impl Llama {
    fn completion_request(
        &self,
        messages: &[Value],
        tools: &[Value],
        stream: bool,
    ) -> RequestBuilder {
        let mut payload = self.payload(messages, tools);
        payload["stream"] = json!(stream);
        payload["temperature"] = json!(self.cfg.temperature);
        payload["max_tokens"] = json!(self.cfg.max_output);
        self.post("/v1/chat/completions", &payload)
    }
}
impl ModelProvider for Llama {
    type Stream = LlamaStream;
    async fn health(&self) -> Health {
        let state = |state, diagnostic, reachable| Health {
            model: self.cfg.model.clone(),
            state,
            diagnostic,
            reachable,
        };
        let offline = "Start llama-server on the configured local address.";
        let short = Duration::from_secs(2);
        let Ok(r) = self.get("/health").timeout(short).send().await else {
            return state("offline", offline, false);
        };
        if r.status() == StatusCode::SERVICE_UNAVAILABLE {
            return state(
                "loading",
                "The model server is loading or temporarily unavailable.",
                true,
            );
        }
        if !r.status().is_success() {
            return state("offline", "llama-server did not report ready.", true);
        }
        let health = bounded_body(r, 65536)
            .await
            .ok()
            .and_then(|d| serde_json::from_slice::<Value>(&d).ok());
        if !health.is_some_and(|v| v["status"] == "ok") {
            return state("offline", "llama-server did not report ready.", true);
        }
        let Ok(r) = self.get("/v1/models").timeout(short).send().await else {
            return state("offline", offline, false);
        };
        if r.status().is_success() {
            let records = bounded_body(r, 1048576)
                .await
                .ok()
                .and_then(|d| serde_json::from_slice::<Value>(&d).ok());
            let Some(list) = records.as_ref().and_then(|v| v["data"].as_array()) else {
                return state("offline", offline, true);
            };
            if list.iter().any(|v| {
                v["id"] == self.cfg.model
                    || v["aliases"]
                        .as_array()
                        .is_some_and(|a| a.iter().any(|s| s == &self.cfg.model))
            }) {
                return state("online", "Local inference is ready.", true);
            }
        }
        state(
            "misconfigured",
            "The configured model alias is unavailable. Check LLAMA_MODEL.",
            true,
        )
    }
    async fn capacity(&self) -> Result<usize> {
        let health = self.health().await;
        let code = match health.state {
            "online" => None,
            "loading" => Some("model_loading"),
            "misconfigured" => Some("model_misconfigured"),
            _ => Some("model_offline"),
        };
        if let Some(code) = code {
            return Err(Error::new(
                code,
                health.diagnostic,
                StatusCode::SERVICE_UNAVAILABLE,
            ));
        }
        let p = document(self.get("/props").timeout(Duration::from_secs(3))).await?;
        let caps = &p["chat_template_caps"];
        if caps["supports_tools"] == false || caps["supports_tool_calls"] == false {
            return Err(Error::bad_turn(
                "tool_template",
                "The loaded template does not support tools. Select a tool-aware GGUF or template.",
            ));
        }
        p["default_generation_settings"]["n_ctx"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| *n > 0)
            .ok_or_else(Error::protocol)
    }
    async fn prompt_tokens(&self, messages: &[Value], tools: &[Value]) -> Result<usize> {
        let rendered =
            document(self.post("/apply-template", &self.payload(messages, tools))).await?;
        let prompt = rendered["prompt"].as_str().ok_or_else(Error::protocol)?;
        let tokens = document(self.post(
            "/tokenize",
            &json!({"content":prompt,"parse_special":true,"add_special":true}),
        ))
        .await?;
        tokens["tokens"]
            .as_array()
            .map(Vec::len)
            .ok_or_else(Error::protocol)
    }
    async fn complete(&self, messages: &[Value], tools: &[Value]) -> Result<(String, Vec<Call>)> {
        let data = document(self.completion_request(messages, tools, false)).await?;
        let choice = &data["choices"][0];
        let message = choice["message"].as_object().ok_or_else(Error::protocol)?;
        if optional_str(message, "role")? != Some("assistant") {
            return Err(Error::protocol());
        }
        let mut parts = BTreeMap::new();
        if let Some(calls) = message.get("tool_calls").filter(|v| !v.is_null()) {
            for (index, call) in calls
                .as_array()
                .ok_or_else(Error::protocol)?
                .iter()
                .enumerate()
            {
                collect_call(&mut parts, call, index)?;
            }
        }
        let text = optional_str(message, "content")?.unwrap_or("").to_owned();
        let calls = finish_calls(choice["finish_reason"].as_str(), parts)?;
        Ok((text, calls))
    }
    async fn stream(&self, messages: &[Value], tools: &[Value]) -> Result<LlamaStream> {
        let response = checked(self.completion_request(messages, tools, true)).await?;
        Ok(LlamaStream {
            response,
            buffer: Vec::new(),
            cursor: 0,
            finish: None,
            parts: BTreeMap::new(),
            reasoning: String::new(),
            size: 0,
            ended: false,
        })
    }
}
pub struct LlamaStream {
    response: Response,
    buffer: Vec<u8>,
    cursor: usize,
    finish: Option<String>,
    parts: BTreeMap<usize, Call>,
    reasoning: String,
    size: usize,
    ended: bool,
}
fn optional_str<'a>(object: &'a Map<String, Value>, key: &str) -> Result<Option<&'a str>> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(v)) => Ok(Some(v)),
        _ => Err(Error::protocol()),
    }
}
impl LlamaStream {
    async fn line(&mut self) -> Result<Option<String>> {
        let mut line = Vec::new();
        loop {
            let pending = &self.buffer[self.cursor..];
            let end = pending.iter().position(|&b| b == b'\n');
            let size = end.unwrap_or(pending.len());
            if line.len() + size > 128 * 1024 {
                return Err(Error::protocol());
            }
            line.extend_from_slice(&pending[..size]);
            self.cursor += size;
            if end.is_some() {
                self.cursor += 1;
                break;
            }
            match self.response.chunk().await.map_err(transport)? {
                Some(chunk) => {
                    self.buffer = chunk.to_vec();
                    self.cursor = 0;
                }
                None if line.is_empty() => return Ok(None),
                None => break,
            }
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        String::from_utf8(line)
            .map(Some)
            .map_err(|_| Error::protocol())
    }
    fn finished(&mut self) -> Result<Part> {
        self.ended = true;
        let calls = finish_calls(self.finish.as_deref(), std::mem::take(&mut self.parts))?;
        Ok(Part::Finished {
            calls,
            reasoning: std::mem::take(&mut self.reasoning),
        })
    }
}
fn collect_call(
    parts: &mut BTreeMap<usize, Call>,
    part: &Value,
    default_index: usize,
) -> Result<()> {
    let part = part.as_object().ok_or_else(Error::protocol)?;
    let index = match part.get("index") {
        None if default_index < 4 => default_index,
        None => return Err(Error::protocol()),
        Some(i) => i.as_u64().filter(|n| *n < 4).ok_or_else(Error::protocol)? as usize,
    };
    if part.get("type").is_some_and(|v| v != "function") {
        return Err(Error::protocol());
    }
    let slot = parts.entry(index).or_default();
    if let Some(id) = optional_str(part, "id")? {
        slot.id.push_str(id);
    }
    if let Some(f) = part.get("function").filter(|v| !v.is_null()) {
        let f = f.as_object().ok_or_else(Error::protocol)?;
        if let Some(n) = optional_str(f, "name")? {
            slot.name.push_str(n);
        }
        match f.get("arguments") {
            Some(Value::Object(obj)) => slot.arguments.push_str(&json!(obj).to_string()),
            Some(Value::String(s)) => slot.arguments.push_str(s),
            None | Some(Value::Null) => (),
            _ => return Err(Error::protocol()),
        }
    }
    if slot.name.chars().count() > 160
        || slot.id.chars().count() > 160
        || slot.arguments.chars().count() > 2048
    {
        return Err(Error::protocol());
    }
    Ok(())
}

fn finish_calls(finish: Option<&str>, parts: BTreeMap<usize, Call>) -> Result<Vec<Call>> {
    match finish {
        Some("length") => {
            return Err(Error::bad_turn(
                "output_limit",
                "The response reached the output token limit. Try a shorter question.",
            ));
        }
        Some("stop") => (),
        Some("tool_calls") if !parts.is_empty() => (),
        _ => return Err(Error::protocol()),
    }
    let mut ids = HashSet::new();
    let mut calls = Vec::new();
    for mut c in parts.into_values() {
        if c.id.is_empty() {
            c.id = format!("call_{}", Uuid::new_v4().simple());
        }
        if c.name.is_empty() || !ids.insert(c.id.clone()) {
            return Err(Error::protocol());
        }
        calls.push(c);
    }
    Ok(calls)
}
impl ModelStream for LlamaStream {
    async fn next(&mut self) -> Result<Part> {
        if self.ended {
            return Err(Error::protocol());
        }
        loop {
            let Some(line) = self.line().await? else {
                return self.finished();
            };
            if line.is_empty() || line.starts_with(':') {
                continue;
            }
            let data = line
                .strip_prefix("data:")
                .ok_or_else(Error::protocol)?
                .trim();
            if data == "[DONE]" {
                return self.finished();
            }
            let packet: Value = serde_json::from_str(data).map_err(|_| Error::protocol())?;
            if packet.get("error").is_some() {
                return Err(Error::protocol());
            }
            let choices = packet["choices"].as_array().ok_or_else(Error::protocol)?;
            if choices.is_empty() && packet.get("usage").is_some() {
                continue;
            }
            if self.finish.is_some() {
                return Err(Error::protocol());
            }
            let choice = choices
                .first()
                .and_then(Value::as_object)
                .ok_or_else(Error::protocol)?;
            self.finish = optional_str(choice, "finish_reason")?.map(str::to_owned);
            let Some(delta) = choice.get("delta").filter(|v| !v.is_null()) else {
                continue;
            };
            let delta = delta.as_object().ok_or_else(Error::protocol)?;
            if let Some(thought) =
                optional_str(delta, "reasoning_content")?.or(optional_str(delta, "reasoning")?)
            {
                self.reasoning.push_str(thought);
            }
            if let Some(parts) = delta.get("tool_calls").filter(|v| !v.is_null()) {
                for part in parts.as_array().ok_or_else(Error::protocol)? {
                    collect_call(&mut self.parts, part, 0)?;
                }
            }
            let content = optional_str(delta, "content")?.unwrap_or("");
            self.size += content.len();
            if self.size + self.reasoning.len() > 2 * 1024 * 1024 {
                return Err(Error::protocol());
            }
            if !content.is_empty() {
                return Ok(Part::Text(content.into()));
            }
        }
    }
}
