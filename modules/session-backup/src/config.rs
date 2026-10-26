//! What a run is told, from the JSON file its module writes.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

/// Where Drive's API is, unless a test points elsewhere.
const API: &str = "https://www.googleapis.com";
/// Where a refresh token is traded for an access token.
const TOKEN: &str = "https://oauth2.googleapis.com/token";
/// How long a log goes unwritten before it counts as settled.
const SETTLED_HOURS: u64 = 48;
/// How long to wait before each retry of a call Drive was too busy for:
/// about a minute in all, well inside the hour before the next run.
const RETRY_SECONDS: [u64; 3] = [2, 10, 30];

/// One run's settings.
#[derive(Debug, Clone)]
pub struct Config {
    /// The folder pi keeps its session logs in, a folder per directory.
    pub sessions: PathBuf,
    /// The ledger of what was sent, and beside it the scratch a seal is
    /// written to before it goes.
    pub ledger: PathBuf,
    /// The Drive folder every object goes in, by its id.
    pub folder: String,
    /// The machine, as its objects are named.
    pub host: String,
    /// Who can open an object: age recipients, plugins' included.
    pub recipients: Vec<String>,
    /// The gcloud application-default login.
    pub credentials: PathBuf,
    pub settled: Duration,
    /// How long a log Drive holds is kept on the machine after its last
    /// write, or `None` to keep every log.
    pub keep: Option<Duration>,
    /// The wait before each retry of a call Drive was busy for.
    pub retries: Vec<Duration>,
    pub api: String,
    pub token: String,
}

impl Config {
    /// The settings in the JSON file at `path`.
    pub fn read(path: &Path) -> Result<Config, String> {
        let text = fs::read_to_string(path)
            .map_err(|why| format!("can't read the settings in {}: {why}", path.display()))?;
        let value: Value = serde_json::from_str(&text)
            .map_err(|why| format!("the settings in {} aren't JSON: {why}", path.display()))?;
        Config::from_json(&value)
    }

    /// The settings a JSON object gives.
    pub fn from_json(value: &Value) -> Result<Config, String> {
        let text = |key: &str| -> Result<String, String> {
            value
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| format!("the settings say no {key}"))
        };
        let recipients: Vec<String> = value
            .get("recipients")
            .and_then(Value::as_array)
            .map(|all| {
                all.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        if recipients.is_empty() {
            return Err("the settings name no recipient, so nothing could open a backup".into());
        }
        let hours = value
            .get("settledHours")
            .and_then(Value::as_u64)
            .unwrap_or(SETTLED_HOURS);
        let keep = value
            .get("keepDays")
            .and_then(Value::as_u64)
            .filter(|days| *days > 0)
            .map(|days| Duration::from_hours(days * 24));
        let retries = value
            .get("retrySeconds")
            .and_then(Value::as_array)
            .map_or_else(
                || RETRY_SECONDS.to_vec(),
                |all| all.iter().filter_map(Value::as_u64).collect(),
            )
            .into_iter()
            .map(Duration::from_secs)
            .collect();
        Ok(Config {
            sessions: text("sessions")?.into(),
            ledger: text("ledger")?.into(),
            folder: text("folder")?,
            host: text("host")?,
            recipients,
            credentials: text("credentials")?.into(),
            settled: Duration::from_hours(hours),
            keep,
            retries,
            api: text("api").unwrap_or_else(|_| API.into()),
            token: text("token").unwrap_or_else(|_| TOKEN.into()),
        })
    }
}
