use axum::{
    Json, Router,
    body::{Body, Bytes},
    response::IntoResponse,
    routing::post,
};
use odyssey_backend::{
    config::Config,
    provider::{Llama, ModelProvider, ModelStream, Part},
};
use serde_json::{Value, json};
use std::{collections::HashMap, convert::Infallible, path::Path, sync::Arc, time::Duration};
use tokio::{net::TcpListener, time::sleep};
use tokio_stream::wrappers::ReceiverStream;

async fn provider(router: Router) -> (Llama, tokio::task::JoinHandle<()>) {
    let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let vars = HashMap::from([(
        "LLAMA_BASE_URL".to_owned(),
        format!("http://{}/v1", socket.local_addr().unwrap()),
    )]);
    let client = Llama::new(Arc::new(
        Config::from_values(Path::new("."), &vars).unwrap(),
    ))
    .unwrap();
    let task = tokio::spawn(async move {
        axum::serve(socket, router).await.unwrap();
    });
    (client, task)
}
#[tokio::test]
async fn utf8_and_lines_split_across_network_reads() {
    let router=Router::new().route("/v1/chat/completions",post(||async {
        let (tx,rx)=tokio::sync::mpsc::channel(1);
        tokio::spawn(async move{
            let body=": ping\r\n\r\ndata: {\"choices\":[{\"delta\":{\"content\":\"天🌲\"},\"finish_reason\":\"stop\"}]}\r\n\r\ndata: [DONE]";
            for part in body.as_bytes().chunks(3){
                if tx.send(Ok::<_,Infallible>(Bytes::copy_from_slice(part))).await.is_err(){break;}
                sleep(Duration::from_millis(1)).await;
            }
        });
        Body::from_stream(ReceiverStream::new(rx))
    }));
    let (model, server) = provider(router).await;
    let mut stream = model.stream(&[], &[]).await.unwrap();
    assert!(matches!(stream.next().await.unwrap(),Part::Text(s) if s=="天🌲"));
    assert!(matches!(stream.next().await.unwrap(),Part::Finished{calls,..} if calls.is_empty()));
    server.abort();
}
#[tokio::test]
async fn standard_non_stream_completion_and_bad_tool_fragments() {
    let route = post(|Json(data): Json<Value>| async move {
        if data["stream"] == false {
            return Json(json!({"choices":[{"finish_reason":"tool_calls","message":{"role":"assistant","content":null,"tool_calls":[
                {"id":"a","function":{"name":"calculator","arguments":{"expression":"2+2"}}},
                {"function":{"name":"get_current_datetime","arguments":"{}"}}]}}]})).into_response();
        }
        let parts = data["messages"][0]["parts"].clone();
        format!(
            "data: {}\n\n",
            json!({"choices":[{"delta":{"tool_calls":parts},"finish_reason":"tool_calls"}]})
        )
        .into_response()
    });
    let (model, server) = provider(Router::new().route("/v1/chat/completions", route)).await;
    let (_, calls) = model.complete(&[], &[]).await.unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].id, "a");
    assert_eq!(calls[0].parsed().unwrap()["expression"], "2+2");
    assert!(calls[1].id.starts_with("call_"));
    for parts in [
        json!([{"index":4}]),
        json!([{"function":{"name":9}}]),
        json!([{"type":"shell"}]),
        json!([{"function":{"arguments":"{}"}}]),
        json!([{"function":{"name":"x","arguments":"X".repeat(2049)}}]),
        json!([{"index":0,"id":"a","function":{"name":"x"}},{"index":1,"id":"a","function":{"name":"y"}}]),
    ] {
        let mut stream = model.stream(&[json!({"parts":parts})], &[]).await.unwrap();
        assert!(matches!(stream.next().await,Err(e) if e.code=="model_protocol"));
    }
    server.abort();
}
