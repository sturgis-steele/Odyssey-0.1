use odyssey_backend::{config::Config, database::Store, tools::Registry};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::{collections::HashMap, path::Path, time::Duration};

#[test]
fn local_configuration_and_clock() {
    for (key, value) in [
        ("BACKEND_HOST", "0.0.0.0"),
        ("LLAMA_BASE_URL", "https://localhost/v1"),
        ("LLAMA_BASE_URL", "http://name@localhost:8080/v1"),
        ("LLAMA_BASE_URL", "http://localhost:99999/v1"),
        ("LLAMA_BASE_URL", "http://example.test/v1"),
        ("MAX_TOOL_ITERATIONS", "0"),
        ("TEMPERATURE", "NaN"),
        ("HISTORY_TURNS", "51"),
        ("MAX_OUTPUT_TOKENS", "4096"),
        ("MODEL_TIMEOUT_SECONDS", "inf"),
        ("ODYSSEY_TIMEZONE", "invalid"),
    ] {
        assert!(
            Config::from_values(Path::new("."), &HashMap::from([(key.into(), value.into())]))
                .is_err(),
            "{key}"
        );
    }
    let settings = Config::from_values(
        Path::new("."),
        &HashMap::from([
            ("LLAMA_BASE_URL".into(), "http://[::1]:8080/v1/".into()),
            ("ODYSSEY_TIMEZONE".into(), "Asia/Kathmandu".into()),
        ]),
    )
    .unwrap();
    let r = Registry::new(settings.timezone);
    let t = r.run("get_current_datetime", Some(&json!({})));
    assert_eq!(t["result"]["utc_offset"], "+0545");
    assert!(
        t["result"]["iso_datetime"]
            .as_str()
            .unwrap()
            .parse::<jiff::Timestamp>()
            .is_ok()
    );
    assert_eq!(
        r.run("get_current_datetime", Some(&json!({"timezone":"UTC"})))["ok"],
        false
    );
    let os = r.run("get_system_information", Some(&json!({})));
    assert_eq!(os["ok"], true);
    assert_eq!(os["result"].as_object().unwrap().len(), 4);
}
#[test]
fn arithmetic_grammar_exactness_and_bounds() {
    let r = Registry::new(None);
    let run = |s: &str| r.run("calculator", Some(&json!({"expression":s})));
    for (expr, result) in [
        ("157*243", "38151"),
        ("(1 # increment\n+ 2)", "3"),
        ("-7%3", "2"),
        ("7%-3", "-2"),
        ("-7//3", "-3"),
        ("2**3**2", "512"),
        ("-2**2", "-4"),
        ("2**-2", "0.25"),
        ("sqrt(144)", "12.0"),
        ("1.0//0.1", "9.0"),
        ("0x10+0b10+1_000", "1018"),
        ("999999999999999999999999+1", "1000000000000000000000000"),
    ] {
        assert_eq!(run(expr)["result"]["value"].to_string(), result, "{expr}");
    }
    for expr in [
        "1/0",
        "sqrt(-1)",
        "type(1)",
        "[]",
        "2**101",
        "9e200",
        "(-1)**0.5",
        "2+",
        "1__5",
        "sqrt(1,2)",
        "0**-1",
        "1e",
        "1;2",
    ] {
        assert_eq!(run(expr)["ok"], false, "{expr}");
    }
    assert_eq!(run(&"1+".repeat(40))["ok"], false);
    for args in [
        json!({}),
        json!({"expression":42}),
        json!({"expression":"1","extra":true}),
        Value::Null,
    ] {
        assert_eq!(r.run("calculator", Some(&args))["ok"], false);
    }
}
#[tokio::test]
async fn v1_lock_recovery_foreign_keys_and_atomic_snapshots() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let store = Store::open(path.clone()).await.unwrap();
    assert!(Store::open(path.clone()).await.is_err());
    let id = store.create().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let tid = store.begin(id.clone(), "user".into()).await.unwrap();
    let (a, b) = tokio::join!(
        store.begin(id.clone(), "other".into()),
        store.delete(id.clone())
    );
    assert_eq!(a.unwrap_err().code, "busy");
    assert_eq!(b.unwrap_err().code, "busy");
    store
        .save(
            tid.clone(),
            "part".into(),
            vec![json!({"call_id":"x","name":"calculator","ok":true,"result":{"value":2}})],
            vec![json!({"role":"assistant","content":"s","reasoning_content":"private"})],
            "generating",
            None,
        )
        .await
        .unwrap();
    drop(store);
    let reopened = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Ok(s) = Store::open(path.clone()).await {
                break s;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let detail = reopened.detail(id.clone(), 20, i64::MAX).await.unwrap();
    let t = &detail["turns"][0];
    assert_eq!(t["id"], tid);
    assert_eq!(t["status"], "interrupted");
    assert_eq!(t["assistant"], "part");
    assert_eq!(t["tools"][0]["call_id"], "x");
    let inspect = Connection::open(&path).unwrap();
    let raw: String = inspect
        .query_row("SELECT transcript FROM turns", [], |r| r.get(0))
        .unwrap();
    assert!(!raw.contains("private"));
    assert_eq!(
        inspect
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        inspect
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        inspect
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
                .get::<_, i32>(
                0
            ))
            .unwrap(),
        0
    );
    inspect.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert_eq!(reopened.create().await.unwrap_err().code, "storage");
    inspect.execute_batch("ROLLBACK").unwrap();
    assert!(reopened.history(id.clone(), 8).await.unwrap().is_empty());
    reopened.delete(id).await.unwrap();
    assert_eq!(
        inspect
            .query_row("SELECT count(*) FROM turns", [], |r| r.get::<_, i32>(0))
            .unwrap(),
        0
    );
}
#[tokio::test]
async fn migration_rollback_and_newer_database_refusal() {
    let dir = tempfile::tempdir().unwrap();
    for version in [0, 2] {
        let path = dir.path().join(format!("db{version}"));
        let db = Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE turns (preserve TEXT); INSERT INTO turns VALUES ('keep');")
            .unwrap();
        db.pragma_update(None, "user_version", version).unwrap();
        assert!(Store::open(path).await.is_err());
        assert_eq!(
            db.query_row("SELECT preserve FROM turns", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "keep"
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='conversations'",
                [],
                |r| r.get::<_, i32>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            version
        );
    }
}
