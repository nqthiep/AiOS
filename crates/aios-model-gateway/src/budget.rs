//! Per-user cloud budget (design.md §18.2).
//!
//! Cost is recorded in `<users-dir>/<uid>/usage.json`, per UTC day.
//! Local models do not count against the cloud budget.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// `usage.json` format version.
pub const USAGE_FORMAT_VERSION: u32 = 1;

/// Cost and tokens for one UTC day.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UsageEntry {
    /// Total cost (USD).
    #[serde(default)]
    pub usd: f64,
    /// Billed requests.
    #[serde(default)]
    pub requests: u64,
    /// Input tokens.
    #[serde(default)]
    pub prompt_tokens: u64,
    /// Output tokens.
    #[serde(default)]
    pub completion_tokens: u64,
}

/// Contents of `usage.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageFile {
    /// Format version.
    #[serde(default = "default_version")]
    pub version: u32,
    /// Cost per UTC day (`YYYY-MM-DD`).
    #[serde(default)]
    pub days: BTreeMap<String, UsageEntry>,
}

fn default_version() -> u32 {
    USAGE_FORMAT_VERSION
}

impl Default for UsageFile {
    fn default() -> Self {
        Self {
            version: USAGE_FORMAT_VERSION,
            days: BTreeMap::new(),
        }
    }
}

impl UsageFile {
    /// Total cost of one day.
    pub fn spent_on(&self, day: &str) -> f64 {
        self.days.get(day).map_or(0.0, |e| e.usd)
    }

    /// Total cost of one month (`YYYY-MM`).
    pub fn spent_in_month(&self, month: &str) -> f64 {
        self.days
            .iter()
            .filter(|(day, _)| day.starts_with(month))
            .map(|(_, e)| e.usd)
            .sum()
    }
}

/// Cost ledger for all users (cached in memory, written to disk on change).
#[derive(Debug)]
pub struct UsageStore {
    users_dir: PathBuf,
    cache: Mutex<HashMap<u32, UsageFile>>,
}

impl UsageStore {
    pub fn new(users_dir: PathBuf) -> Self {
        Self {
            users_dir,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Path of a user's `usage.json`.
    pub fn usage_path(&self, uid: u32) -> PathBuf {
        self.users_dir.join(uid.to_string()).join("usage.json")
    }

    /// A user's `usage.json` (read from disk once, then cached).
    pub fn load(&self, uid: u32) -> UsageFile {
        let mut guard = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(file) = guard.get(&uid) {
            return file.clone();
        }
        let file = read_usage_file(&self.usage_path(uid)).unwrap_or_default();
        guard.insert(uid, file.clone());
        file
    }

    /// Spent today (UTC).
    pub fn spent_today(&self, uid: u32) -> f64 {
        self.load(uid).spent_on(&utc_today())
    }

    /// Spent this month (UTC).
    pub fn spent_this_month(&self, uid: u32) -> f64 {
        self.load(uid).spent_in_month(&utc_month())
    }

    /// Add the cost of one cloud request to the user's ledger.
    // ponytail: check-then-record is not atomic, so concurrent requests can overshoot
    // the budget by the in-flight requests; reserve cost up front if that matters.
    pub fn record(&self, uid: u32, cost_usd: f64, prompt_tokens: u64, completion_tokens: u64) {
        let day = utc_today();
        let path = self.usage_path(uid);
        let file = {
            let mut guard = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            let entry = guard.entry(uid).or_default();
            let slot = entry.days.entry(day.clone()).or_default();
            slot.usd += cost_usd;
            slot.requests += 1;
            slot.prompt_tokens += prompt_tokens;
            slot.completion_tokens += completion_tokens;
            entry.clone()
        };
        if let Err(error) = write_usage_file(&path, &file) {
            tracing::error!(uid, error = %error, "cannot write usage file");
        }
    }
}

/// Read `usage.json`; a corrupt file counts as empty (does not block requests).
fn read_usage_file(path: &Path) -> Option<UsageFile> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<UsageFile>(&text) {
        Ok(file) => Some(file),
        Err(error) => {
            tracing::error!(path = %path.display(), error = %error, "cannot parse usage file");
            None
        }
    }
}

/// Write `usage.json` (temp file + rename, so it is never half-written).
fn write_usage_file(path: &Path, file: &UsageFile) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(file)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

/// Current UTC date, `YYYY-MM-DD`.
pub fn utc_today() -> String {
    let (year, month, day) = utc_civil_date();
    format!("{year:04}-{month:02}-{day:02}")
}

/// Current UTC month, `YYYY-MM`.
pub fn utc_month() -> String {
    let (year, month, _) = utc_civil_date();
    format!("{year:04}-{month:02}")
}

/// Current UTC civil date (year, month, day).
fn utc_civil_date() -> (i64, u32, u32) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    civil_from_days((secs / 86_400) as i64)
}

/// Convert days since 1970-01-01 to a Gregorian (year, month, day).
/// Howard Hinnant's `civil_from_days` algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates_are_correct() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(1), (1970, 1, 2));
        assert_eq!(civil_from_days(59), (1970, 3, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        assert_eq!(civil_from_days(20_727), (2026, 10, 1));
        assert_eq!(civil_from_days(20_747), (2026, 10, 21));
    }

    #[test]
    fn usage_file_sums_by_day_and_month() {
        let mut file = UsageFile::default();
        file.days.insert(
            "2026-10-20".to_string(),
            UsageEntry {
                usd: 1.0,
                requests: 1,
                prompt_tokens: 10,
                completion_tokens: 20,
            },
        );
        file.days.insert(
            "2026-10-21".to_string(),
            UsageEntry {
                usd: 2.0,
                requests: 1,
                prompt_tokens: 5,
                completion_tokens: 6,
            },
        );
        file.days.insert(
            "2026-09-30".to_string(),
            UsageEntry {
                usd: 4.0,
                requests: 1,
                prompt_tokens: 1,
                completion_tokens: 1,
            },
        );
        assert_eq!(file.spent_on("2026-10-21"), 2.0);
        assert_eq!(file.spent_in_month("2026-10"), 3.0);
        assert_eq!(file.spent_in_month("2026-09"), 4.0);
    }
}
