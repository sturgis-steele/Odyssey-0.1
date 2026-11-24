use crate::{
    config::Config,
    database::{INTERRUPTED, Store},
    error::{Error, Result},
    provider::{ModelProvider, ModelStream, Part},
    tools::Registry,
};
use axum::response::sse::Event;
use serde_json::{Value, json};
use std::{collections::VecDeque, time::Duration};
use tokio::{
    sync::{OwnedSemaphorePermit, mpsc, oneshot},
    time::Instant,
};

pub(crate) type Events = mpsc::Receiver<Event>;
struct Turn {
    cid: String,
    tid: String,
    transcript: Vec<Value>,
    activity: Vec<Value>,
    text: String,
}
impl Turn {
    async fn save(
        &self,
        store: &Store,
        status: &'static str,
        error: Option<&'static str>,
    ) -> Result<()> {
        store
            .save(
                self.tid.clone(),
                self.text.clone(),
                self.activity.clone(),
                self.transcript.clone(),
                status,
                error,
            )
            .await
    }
    async fn generate(
        &mut self,
        cfg: &Config,
        store: &Store,
        model: &impl ModelProvider,
        registry: &Registry,
        out: &mpsc::Sender<Event>,
    ) -> Result<()> {
        emit(
            out,
            "message_start",
            json!({"turn_id":self.tid,"conversation_id":self.cid}),
        )
        .await;
        let mut history = VecDeque::from(store.history(self.cid.clone(), cfg.history).await?);
        let tools = registry.definitions();
        let capacity = cfg.context.min(model.capacity().await?);
        let mut checkpoint = Instant::now();
        for iteration in 0..=cfg.max_rounds {
            let messages = loop {
                let mut messages = vec![json!({"role":"system","content":cfg.prompt})];
                messages.extend(history.iter().flatten().cloned());
                messages.extend(self.transcript.iter().cloned());
                if model.prompt_tokens(&messages, tools).await? + cfg.max_output + 8 <= capacity {
                    break messages;
                }
                if history.pop_front().is_none() {
                    return Err(Error::bad_turn(
                        "context_limit",
                        "This turn exceeds the available context. Try a shorter message.",
                    ));
                }
            };
            let mut stream = model.stream(&messages, tools).await?;
            let mut round_text = String::new();
            let (calls, reasoning) = loop {
                match stream.next().await? {
                    Part::Text(delta) => {
                        round_text.push_str(&delta);
                        self.text.push_str(&delta);
                        if checkpoint.elapsed() >= Duration::from_secs(1) {
                            self.save(store, "generating", None).await?;
                            checkpoint = Instant::now();
                        }
                        emit(out, "text_delta", json!({"text":delta})).await;
                    }
                    Part::Finished { calls, reasoning } => break (calls, reasoning),
                }
            };
            drop(stream);
            let mut assistant = json!({"role":"assistant","content":round_text});
            if calls.is_empty() {
                if round_text.trim().is_empty()
                    || ["<tool_call>", "<|tool_call_start|>"]
                        .iter()
                        .any(|m| round_text.contains(m))
                {
                    return Err(Error::new(
                        "tool_template",
                        "Expected a structured model response. Check the tool-aware chat template.",
                        axum::http::StatusCode::BAD_GATEWAY,
                    ));
                }
                self.transcript.push(assistant);
                return Ok(());
            }
            if iteration == cfg.max_rounds {
                return Err(Error::bad_turn(
                    "tool_limit",
                    "The model reached the tool-call limit. Try a more specific request.",
                ));
            }
            assistant["tool_calls"] = Value::Array(calls.iter().map(|c| c.wire()).collect());
            if !reasoning.is_empty() {
                assistant["reasoning_content"] = json!(reasoning);
            }
            self.transcript.push(assistant);
            for call in calls {
                emit(
                    out,
                    "tool_start",
                    json!({"call_id":call.id,"name":call.name}),
                )
                .await;
                let reg = registry.clone();
                let name = call.name.clone();
                let args = call.parsed();
                let outcome = tokio::task::spawn_blocking(move || reg.run(&name, args.as_ref()))
                    .await
                    .map_err(|_| internal())?;
                self.transcript.push(json!({"role":"tool","name":call.name,"tool_call_id":call.id,"content":outcome.to_string()}));
                let mut activity = outcome;
                activity["call_id"] = json!(call.id);
                activity["name"] = json!(call.name);
                self.activity.push(activity.clone());
                self.save(store, "generating", None).await?;
                emit(out, "tool_result", activity).await;
            }
            if !round_text.is_empty() {
                self.text.push_str("\n\n");
                emit(out, "text_delta", json!({"text":"\n\n"})).await;
            }
        }
        Err(internal())
    }
}
fn internal() -> Error {
    Error::new(
        "internal",
        "The response could not be completed. Try again.",
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    )
}
async fn emit(out: &mpsc::Sender<Event>, name: &str, data: Value) {
    let _ = out
        .send(Event::default().event(name).data(data.to_string()))
        .await;
}
pub(crate) async fn start(
    state: crate::api::StateData,
    permit: OwnedSemaphorePermit,
    cid: String,
    message: String,
) -> Result<Events> {
    let (out, events) = mpsc::channel(8);
    let (ack, ready) = oneshot::channel();
    // The worker owns begin + finalization even if the HTTP handler disappears before headers.
    tokio::spawn(async move {
        let tid = match state.store.begin(cid.clone(), message.clone()).await {
            Ok(id) => id,
            Err(e) => {
                let _ = ack.send(Err(e));
                return;
            }
        };
        let mut turn = Turn {
            cid,
            tid,
            transcript: vec![json!({"role":"user","content":message})],
            activity: Vec::new(),
            text: String::new(),
        };
        let _ = ack.send(Ok(()));
        let result = tokio::select! {
            biased;
            _=out.closed()=>None,
            result=turn.generate(&state.cfg,&state.store,&state.model,&state.tools,&out)=>Some(result),
        };
        let (status, detail, mut terminal) = match result {
            None => ("interrupted", Some(INTERRUPTED), None),
            Some(Ok(())) => (
                "complete",
                None,
                Some(("message_complete", json!({"turn_id":turn.tid}))),
            ),
            Some(Err(e)) => {
                tracing::warn!(kind = e.code, "turn_failed");
                (
                    "error",
                    Some(e.detail),
                    Some((
                        "error",
                        json!({"turn_id":turn.tid,"code":e.code,"message":e.detail}),
                    )),
                )
            }
        };
        let mut delay = Duration::from_millis(100);
        let mut stream = Some(out);
        let mut status = status;
        let mut detail = detail;
        // Keep ownership until a durable terminal snapshot. A transient lock must not strand a live row.
        while turn.save(&state.store, status, detail).await.is_err() {
            if let Some(out) = stream.take()
                && terminal.take().is_some()
            {
                let e = Error::storage();
                emit(
                    &out,
                    "error",
                    json!({"turn_id":turn.tid,"code":e.code,"message":e.detail}),
                )
                .await;
                status = "error";
                detail = Some(e.detail);
            }
            tokio::time::sleep(delay).await;
            delay = (delay * 2).min(Duration::from_secs(5));
        }
        drop(permit);
        if let (Some(out), Some((kind, data))) = (stream, terminal) {
            emit(&out, kind, data).await;
        }
    });
    ready.await.map_err(|_| internal())??;
    Ok(events)
}
