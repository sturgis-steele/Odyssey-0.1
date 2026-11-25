use jiff::tz::TimeZone;
use reqwest::Url;
use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

pub const DEFAULT_PROMPT: &str = "You are Odyssey, a concise offline assistant on this device. Answer accurately and admit uncertainty. Always call calculator for math; it supports arithmetic, powers, sqrt, trigonometry (sin, cos, tan and inverses), logarithms (ln, log2, log10), exp, abs, floor, ceil and the constants pi and e. Call get_current_datetime for the current time or date, and get_system_information for device details. Use actual tool results. Never invent results. Internet, reference libraries, personal memory, maps and calendar are unavailable.";

#[derive(Clone)]
pub struct Config {
    pub root: PathBuf,
    pub app_name: String,
    pub host: String,
    pub port: u16,
    pub base_url: String,
    pub model: String,
    pub database: PathBuf,
    pub max_rounds: usize,
    pub max_output: usize,
    pub context: usize,
    pub history: usize,
    pub timeout: Duration,
    pub temperature: f64,
    pub thinking: bool,
    pub prompt: String,
    pub timezone: Option<TimeZone>,
    pub log_level: tracing::Level,
}
pub fn root() -> std::io::Result<PathBuf> {
    if let Some(root) = env::var_os("ODYSSEY_ROOT") {
        return PathBuf::from(root).canonicalize();
    }
    let here = env::current_dir()?;
    if here.ends_with("backend-rust")
        && let Some(parent) = here.parent()
    {
        return Ok(parent.into());
    }
    Ok(here)
}
fn bounded<T: FromStr + PartialOrd>(value: &str, key: &str, lo: T, hi: T) -> Result<T, String> {
    value
        .parse::<T>()
        .ok()
        .filter(|v| v >= &lo && v <= &hi)
        .ok_or_else(|| format!("Invalid {key}"))
}
pub fn local_host(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}
impl Config {
    pub fn load(root: PathBuf) -> Result<Self, String> {
        let mut vars = HashMap::new();
        let file = root.join("backend/.env");
        match dotenvy::from_path_iter(&file) {
            Ok(entries) => {
                for entry in entries {
                    let (key, value) = entry.map_err(|_| "Invalid setting in backend/.env")?;
                    vars.insert(key.to_ascii_uppercase(), value);
                }
            }
            Err(dotenvy::Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err("Cannot read backend/.env".into()),
        }
        for (k, v) in env::vars_os() {
            if let (Ok(k), Ok(v)) = (k.into_string(), v.into_string()) {
                vars.insert(k.to_ascii_uppercase(), v);
            }
        }
        Self::from_values(&root, &vars)
    }
    pub fn from_values(root: &Path, vars: &HashMap<String, String>) -> Result<Self, String> {
        let get =
            |key: &str, default: &str| vars.get(key).cloned().unwrap_or_else(|| default.to_owned());
        let text = |key: &str, default: &str, max| {
            let s = get(key, default);
            if s.is_empty() || s.chars().count() > max {
                Err(format!("Invalid {key}"))
            } else {
                Ok(s)
            }
        };
        let app_name = text("APP_NAME", "Odyssey", 64)?;
        if !matches!(
            get("APP_ENV", "development").as_str(),
            "development" | "test" | "production"
        ) {
            return Err("Invalid APP_ENV".into());
        }
        let host = get("BACKEND_HOST", "127.0.0.1");
        if !local_host(&host) {
            return Err("BACKEND_HOST must be loopback".into());
        }
        let raw = get("LLAMA_BASE_URL", "http://127.0.0.1:8080/v1");
        let url = Url::parse(&raw).map_err(|_| "Invalid LLAMA_BASE_URL")?;
        let authority = raw
            .strip_prefix("http://")
            .and_then(|s| s.split('/').next())
            .unwrap_or("");
        let host_text = authority
            .strip_prefix("[::1]")
            .map(|s| ("::1", s))
            .unwrap_or_else(|| {
                authority
                    .split_once(':')
                    .map_or((authority, ""), |(h, _)| (h, &authority[h.len()..]))
            });
        if !local_host(host_text.0)
            || !(host_text.1.is_empty()
                || host_text
                    .1
                    .strip_prefix(':')
                    .is_some_and(|p| p.parse::<u16>().is_ok_and(|n| n > 0)))
            || url.path().trim_end_matches('/') != "/v1"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err("LLAMA_BASE_URL must be a loopback HTTP /v1 endpoint".into());
        }
        let max_output = bounded(
            &get("MAX_OUTPUT_TOKENS", "512"),
            "MAX_OUTPUT_TOKENS",
            32,
            8192,
        )?;
        let context = bounded(
            &get("CONTEXT_TOKENS", "4096"),
            "CONTEXT_TOKENS",
            1024,
            131072,
        )?;
        if max_output + 128 >= context {
            return Err("Reserve prompt space within CONTEXT_TOKENS".into());
        }
        let timeout = bounded(
            &get("MODEL_TIMEOUT_SECONDS", "120"),
            "MODEL_TIMEOUT_SECONDS",
            1.0,
            600.0,
        )?;
        let thinking = match get("ENABLE_THINKING", "false")
            .trim()
            .to_lowercase()
            .as_str()
        {
            "true" | "1" | "yes" | "y" | "on" | "t" => true,
            "false" | "0" | "no" | "n" | "off" | "f" => false,
            _ => return Err("Invalid ENABLE_THINKING".into()),
        };
        let timezone = vars
            .get("ODYSSEY_TIMEZONE")
            .map(|z| TimeZone::get(z).map_err(|_| "Invalid ODYSSEY_TIMEZONE"))
            .transpose()?;
        Ok(Self {
            root: root.into(),
            app_name,
            host,
            port: bounded(&get("BACKEND_PORT", "8000"), "BACKEND_PORT", 1, u16::MAX)?,
            base_url: url.as_str().trim_end_matches('/').into(),
            model: text("LLAMA_MODEL", "odyssey-local", 200)?,
            database: root.join(text("DATABASE_PATH", "data/odyssey.db", 32767)?),
            max_rounds: bounded(
                &get("MAX_TOOL_ITERATIONS", "3"),
                "MAX_TOOL_ITERATIONS",
                1,
                8,
            )?,
            max_output,
            context,
            history: bounded(&get("HISTORY_TURNS", "8"), "HISTORY_TURNS", 0, 50)?,
            timeout: Duration::from_secs_f64(timeout),
            temperature: bounded(&get("TEMPERATURE", "0.1"), "TEMPERATURE", 0.0, 2.0)?,
            thinking,
            prompt: text("SYSTEM_PROMPT", DEFAULT_PROMPT, 4096)?,
            timezone,
            log_level: get("LOG_LEVEL", "info")
                .parse()
                .map_err(|_| "Invalid LOG_LEVEL")?,
        })
    }
}
