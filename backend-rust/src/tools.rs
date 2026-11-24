mod calculator;
use crate::VERSION;
use chrono::{Local, SecondsFormat, Utc};
use chrono_tz::Tz;
use serde_json::{Value, json};

#[derive(Clone)]
pub struct Registry {
    definitions: Vec<Value>,
    timezone: Option<Tz>,
}
impl Registry {
    pub fn new(timezone: Option<Tz>) -> Self {
        Self {
            definitions: serde_json::from_str(include_str!("../tool-definitions.json"))
                .expect("bundled tool definitions"),
            timezone,
        }
    }
    pub fn definitions(&self) -> &[Value] {
        &self.definitions
    }
    pub fn catalog(&self) -> Vec<Value> {
        self.definitions
            .iter()
            .map(|definition| {
                let mut tool = definition["function"].clone();
                tool["permission"] = json!("read-only");
                tool
            })
            .collect()
    }
    pub fn contains(&self, name: &str) -> bool {
        self.definitions
            .iter()
            .any(|d| d["function"]["name"] == name)
    }
    pub fn run(&self, name: &str, input: Option<&Value>) -> Value {
        let invalid = "Arguments must match the tool's JSON schema.";
        let result = match (name, input.and_then(Value::as_object)) {
            ("calculator", Some(args)) if args.len() == 1 => args
                .get("expression")
                .and_then(Value::as_str)
                .filter(|s| (1..=192).contains(&s.chars().count()))
                .ok_or(invalid)
                .and_then(calculator::calculate)
                .map(|v| json!({"value":v})),
            ("get_current_datetime", Some(args)) if args.is_empty() => Ok(self.clock()),
            ("get_system_information", Some(args)) if args.is_empty() => device_info(),
            _ if self.contains(name) => Err(invalid),
            _ => Err("This tool is unavailable."),
        };
        // Use only registered names in logs. All arguments, prompts and results are private.
        tracing::info!(
            tool = if self.contains(name) { name } else { "unknown" },
            ok = result.is_ok(),
            "tool_complete"
        );
        match result {
            Ok(v) => json!({"ok":true,"result":v}),
            Err(e) => json!({"ok":false,"result":{"error":e}}),
        }
    }
    fn clock(&self) -> Value {
        let now = Utc::now();
        let zone = self
            .timezone
            .or_else(|| iana_time_zone::get_timezone().ok()?.parse().ok());
        if let Some(zone) = zone {
            let t = now.with_timezone(&zone);
            clock_fields(
                t.to_rfc3339_opts(SecondsFormat::Secs, false),
                t.format("%Z").to_string(),
                t.format("%z").to_string(),
            )
        } else {
            let t = now.with_timezone(&Local);
            clock_fields(
                t.to_rfc3339_opts(SecondsFormat::Secs, false),
                t.format("%Z").to_string(),
                t.format("%z").to_string(),
            )
        }
    }
}
fn clock_fields(iso: String, timezone: String, offset: String) -> Value {
    json!({"date":&iso[..10],"time":&iso[11..19],"timezone":timezone,"utc_offset":offset,"iso_datetime":iso})
}
#[cfg(windows)]
fn os_release() -> Option<String> {
    use winreg::{RegKey, enums::HKEY_LOCAL_MACHINE};
    let k = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        .ok()?;
    let build: String = k.get_value("CurrentBuildNumber").ok()?;
    let build: u32 = build.parse().ok()?;
    Some(if build >= 22000 {
        "11".into()
    } else if build >= 10240 {
        "10".into()
    } else {
        k.get_value("CurrentVersion").ok()?
    })
}
#[cfg(target_os = "linux")]
fn os_release() -> Option<String> {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .ok()
        .map(|v| v.trim().into())
}
#[cfg(not(any(windows, target_os = "linux")))]
fn os_release() -> Option<String> {
    None
}
fn device_info() -> Result<Value, &'static str> {
    let release = os_release().ok_or("The device could not complete this tool.")?;
    let (os, arch) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => ("Windows", "AMD64"),
        ("windows", "aarch64") => ("Windows", "ARM64"),
        ("linux", arch) => ("Linux", arch),
        other => other,
    };
    Ok(
        json!({"operating_system":os,"architecture":arch,"release":release,"application_version":VERSION}),
    )
}
