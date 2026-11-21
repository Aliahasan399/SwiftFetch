//! Application settings, persisted as one JSON document in the `settings`
//! table under the key `app_settings`.

use serde::{Deserialize, Serialize};

use crate::domain::{
    DEFAULT_CONNECTIONS_PER_TASK, DEFAULT_GLOBAL_SPEED_LIMIT, DEFAULT_MAX_ACTIVE_TASKS,
    DEFAULT_TASK_SPEED_LIMIT, OverwritePolicy,
};

/// Missing fields in an older persisted JSON document fall back to the values
/// in `Default`, so adding a setting never invalidates an existing install.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Root directory for new downloads when a category does not override it.
    pub destination_root: String,
    pub max_active_tasks: u32,
    pub connections_per_task: u32,
    pub global_speed_limit: u64,
    pub default_task_speed_limit: u64,
    pub max_retries: i64,
    pub overwrite_policy: OverwritePolicy,
    pub verify_checksums: bool,
    /// Run a user-configured scanner command before marking a file complete.
    pub scan_with_antivirus: bool,
    pub scanner_command: String,
    pub prevent_sleep: bool,
    pub start_on_login: bool,
    pub theme: String,
    pub retention_days: i64,
    pub browser_integration: bool,
    pub proxy: String,
    pub scheduler_enabled: bool,
    pub shutdown_after_complete: bool,
    /// Keep partial files after a failed download so the user can retry.
    pub keep_partial_on_failure: bool,
    /// BCP-47 locale key, persisted as a key rather than translated text.
    pub locale: String,
    /// Opt-in clipboard URL capture. Off by default: never read the clipboard
    /// unless the user explicitly enables it.
    pub clipboard_capture: bool,
    /// Opt-in at-rest protection of saved request headers (cookies, auth) via
    /// the OS credential store. Off by default so existing plaintext queues
    /// and the wire protocol are unaffected. Enabling it migrates existing
    /// rows through the `encrypt_saved_secrets` command.
    pub encrypt_saved_headers: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            destination_root: default_downloads_dir(),
            max_active_tasks: DEFAULT_MAX_ACTIVE_TASKS,
            connections_per_task: DEFAULT_CONNECTIONS_PER_TASK,
            global_speed_limit: DEFAULT_GLOBAL_SPEED_LIMIT,
            default_task_speed_limit: DEFAULT_TASK_SPEED_LIMIT,
            max_retries: 3,
            overwrite_policy: OverwritePolicy::Rename,
            verify_checksums: true,
            scan_with_antivirus: false,
            scanner_command: String::new(),
            prevent_sleep: false,
            start_on_login: false,
            theme: "dark".to_string(),
            retention_days: 90,
            browser_integration: false,
            proxy: String::new(),
            scheduler_enabled: true,
            shutdown_after_complete: false,
            keep_partial_on_failure: true,
            locale: "en".to_string(),
            clipboard_capture: false,
            encrypt_saved_headers: false,
        }
    }
}

/// Resolve the default download root.
///
/// Precedence — `SWIFTFETCH_DOWNLOADS_DIR` lets an operator send downloads to a
/// drive with room, which matters when `%USERPROFILE%` sits on a nearly full
/// system drive (the default `%USERPROFILE%\Downloads\SwiftFetch` would fill it).
/// Split from the wrapper so the precedence can be unit-tested without mutating
/// the process environment.
fn default_downloads_dir_with(get: impl Fn(&str) -> Option<String>) -> String {
    if let Some(dir) = get("SWIFTFETCH_DOWNLOADS_DIR").filter(|v| !v.trim().is_empty()) {
        return dir;
    }
    if let Some(home) = get("USERPROFILE").filter(|v| !v.trim().is_empty()) {
        return format!("{home}\\Downloads\\SwiftFetch");
    }
    if let Some(home) = get("HOME").filter(|v| !v.trim().is_empty()) {
        return format!("{home}/Downloads/SwiftFetch");
    }
    ".".to_string()
}

fn default_downloads_dir() -> String {
    default_downloads_dir_with(|key| std::env::var(key).ok())
}

#[cfg(test)]
mod default_downloads_dir_tests {
    use super::*;

    /// A stub environment: only the named keys resolve, the rest report unset.
    /// Owns its data so the returned closure needs no lifetime.
    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let owned: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key: &str| owned.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    #[test]
    fn explicit_downloads_dir_wins() {
        let got = default_downloads_dir_with(env(&[
            ("SWIFTFETCH_DOWNLOADS_DIR", r"E:\CloneIdm\swiftfetch\downloads"),
            ("USERPROFILE", r"C:\Users\x"),
        ]));
        assert_eq!(got, r"E:\CloneIdm\swiftfetch\downloads");
    }

    #[test]
    fn blank_downloads_dir_is_ignored() {
        let got = default_downloads_dir_with(env(&[
            ("SWIFTFETCH_DOWNLOADS_DIR", "  "),
            ("USERPROFILE", r"C:\Users\x"),
        ]));
        assert_eq!(got, r"C:\Users\x\Downloads\SwiftFetch");
    }

    #[test]
    fn falls_back_to_userprofile_then_home_then_cwd() {
        let got = default_downloads_dir_with(env(&[
            ("USERPROFILE", r"C:\Users\x"),
            ("HOME", r"/home/x"),
        ]));
        assert_eq!(got, r"C:\Users\x\Downloads\SwiftFetch");

        let got = default_downloads_dir_with(env(&[("HOME", r"/home/x")]));
        assert_eq!(got, r"/home/x/Downloads/SwiftFetch");

        let got = default_downloads_dir_with(env(&[]));
        assert_eq!(got, ".");
    }

    #[test]
    fn older_settings_json_gets_new_defaults() {
        let old = r#"{"destination_root":".","max_active_tasks":2}"#;
        let s = Settings::from_json(old);
        assert_eq!(s.locale, "en");
        assert!(!s.clipboard_capture);
        assert_eq!(s.max_active_tasks, 2);
        assert!(s.verify_checksums);
        // A persisted document predating the flag stays unencrypted.
        assert!(!s.encrypt_saved_headers);
    }

    #[test]
    fn encrypt_saved_headers_round_trips_through_json() {
        let default = Settings::default();
        assert!(!default.encrypt_saved_headers, "encryption must be opt-in");

        let mut on = Settings::default();
        on.encrypt_saved_headers = true;
        let raw = on.to_json();
        assert!(raw.contains("\"encrypt_saved_headers\":true"), "{raw}");
        assert!(Settings::from_json(&raw).encrypt_saved_headers);

        // Unknown/missing fields still normalize to the safe default.
        assert!(!Settings::from_json("{}").encrypt_saved_headers);
    }
}

impl Settings {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn from_json(raw: &str) -> Settings {
        serde_json::from_str(raw).unwrap_or_default()
    }

    /// Clamp values that must stay inside documented bounds.
    pub fn normalized(mut self) -> Settings {
        self.max_active_tasks = self.max_active_tasks.clamp(1, crate::domain::MAX_ACTIVE_TASKS);
        self.connections_per_task =
            self.connections_per_task.clamp(1, crate::domain::MAX_CONNECTIONS_PER_TASK);
        self.max_retries = self.max_retries.clamp(0, 20);
        self.retention_days = self.retention_days.clamp(0, 3650);
        if !matches!(self.locale.as_str(), "en" | "bn") { self.locale = "en".to_string(); }
        self
    }
}
