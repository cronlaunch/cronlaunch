// src/core/plist_io.rs

use crate::core::constants::{launch_agents_dir, INTERVALS, ON_LOGIN};
use anyhow::{Context, Result};
use plist::{Dictionary, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ParsedJob {
    pub label: String,
    pub schedule: String,
    pub command: String,
    pub watch_paths: Vec<String>,
    pub is_login: bool,
}

pub fn value_to_pretty_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Real(f) => f.to_string(),
        Value::Date(d) => format!("{d:?}"),
        Value::Uid(u) => format!("{u:?}"),
        Value::Array(a) => format!("[{} items]", a.len()),
        Value::Dictionary(d) => format!("{{{} keys}}", d.len()),
        Value::Data(bytes) => format!("<data {} bytes>", bytes.len()),

        // plist::Value is #[non_exhaustive], so keep a fallback.
        other => format!("{other:?}"),
    }
}

fn parse_schedule(dict: &Dictionary) -> (String, Vec<String>, bool) {
    if let Some(Value::Array(paths)) = dict.get("WatchPaths") {
        let watch_paths: Vec<String> = paths.iter().map(value_to_pretty_string).collect();
        if !watch_paths.is_empty() {
            return (watch_paths[0].clone(), watch_paths, false);
        }
    }

    // Treat RunAtLoad=true as a login job.
    if let Some(Value::Boolean(true)) = dict.get("RunAtLoad") {
        return (ON_LOGIN.to_string(), vec![], true);
    }

    if let Some(Value::Dictionary(intervals)) = dict.get("StartCalendarInterval") {
        let parts: Vec<String> = INTERVALS
            .iter()
            .map(|k| {
                intervals
                    .get(*k)
                    .map(value_to_pretty_string)
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "*".to_string())
            })
            .collect();
        return (parts.join(" "), vec![], false);
    }

    (ON_LOGIN.to_string(), vec![], false)
}

fn parse_command(dict: &Dictionary, label: &str) -> String {
    if let Some(Value::Array(args)) = dict.get("ProgramArguments") {
        let parts: Vec<String> = args.iter().map(value_to_pretty_string).collect();
        if !parts.is_empty() {
            return parts.join(" ");
        }
    }
    if let Some(v) = dict.get("Program") {
        let s = value_to_pretty_string(v);
        if !s.trim().is_empty() {
            return s;
        }
    }
    label.to_string()
}

pub fn load_plist(path: &Path) -> Result<Dictionary> {
    let val = Value::from_file(path)
        .with_context(|| format!("Failed to read plist: {}", path.display()))?;
    match val {
        Value::Dictionary(d) => Ok(d),
        _ => Ok(Dictionary::new()),
    }
}

pub fn parse_job(path: &Path) -> Result<ParsedJob> {
    let dict = load_plist(path)?;
    let label = dict
        .get("Label")
        .map(value_to_pretty_string)
        .unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });
    let (schedule, watch_paths, is_login) = parse_schedule(&dict);
    let command = parse_command(&dict, &label);

    Ok(ParsedJob {
        label,
        schedule,
        command,
        watch_paths,
        is_login,
    })
}

pub fn list_plists() -> Result<Vec<PathBuf>> {
    let dir = launch_agents_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut out = vec![];
    for entry in
        fs::read_dir(&dir).with_context(|| format!("Failed to read dir: {}", dir.display()))?
    {
        let p = entry?.path();
        if p.extension().map(|e| e == "plist").unwrap_or(false) {
            out.push(p);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use plist::{Dictionary, Value};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    struct HomeGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        old_home: Option<std::ffi::OsString>,
        _td: TempDir,
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            if let Some(old) = self.old_home.take() {
                std::env::set_var("HOME", old);
            } else {
                std::env::remove_var("HOME");
            }
            // TempDir cleans itself up here too.
        }
    }

    fn set_temp_home() -> HomeGuard {
        let lock = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();

        let td = TempDir::new().expect("tempdir");
        let old_home = std::env::var_os("HOME");

        std::env::set_var("HOME", td.path());

        HomeGuard {
            _lock: lock,
            old_home,
            _td: td,
        }
    }

    fn write_plist(label: &str, dict: Dictionary) -> PathBuf {
        let path = launch_agents_dir().join(format!("{label}.plist"));
        fs::create_dir_all(path.parent().expect("parent dir")).expect("mkdir");
        plist::to_file_xml(&path, &Value::Dictionary(dict)).expect("write plist");
        path
    }

    #[test]
    fn value_to_pretty_string_string() {
        let v = Value::String("hello".to_string());
        assert_eq!(value_to_pretty_string(&v), "hello");
    }

    #[test]
    fn parse_job_calendar_interval_program_arguments() {
        let _home = set_temp_home();

        let mut intervals = Dictionary::new();
        intervals.insert("Minute".to_string(), Value::String("0".to_string()));
        intervals.insert("Hour".to_string(), Value::String("1".to_string()));
        intervals.insert("Day".to_string(), Value::String("*".to_string()));
        intervals.insert("Month".to_string(), Value::String("*".to_string()));
        intervals.insert("Weekday".to_string(), Value::String("*".to_string()));

        let mut d = Dictionary::new();
        d.insert(
            "Label".to_string(),
            Value::String("com.local.testjob".to_string()),
        );
        d.insert(
            "StartCalendarInterval".to_string(),
            Value::Dictionary(intervals),
        );
        d.insert(
            "ProgramArguments".to_string(),
            Value::Array(vec![
                Value::String("/bin/echo".to_string()),
                Value::String("hello".to_string()),
            ]),
        );

        let path = write_plist("com.local.testjob", d);
        let job = parse_job(&path).expect("parse_job");

        assert_eq!(job.label, "com.local.testjob");
        assert_eq!(job.schedule, "0 1 * * *");
        assert_eq!(job.command, "/bin/echo hello");
        assert!(job.watch_paths.is_empty());
        assert!(!job.is_login);
    }

    #[test]
    fn parse_job_watch_paths_program() {
        let _home = set_temp_home();

        let mut d = Dictionary::new();
        d.insert(
            "Label".to_string(),
            Value::String("com.local.watch".to_string()),
        );
        d.insert(
            "WatchPaths".to_string(),
            Value::Array(vec![
                Value::String("/tmp/a".to_string()),
                Value::String("/tmp/b".to_string()),
            ]),
        );
        d.insert(
            "Program".to_string(),
            Value::String("/usr/bin/true".to_string()),
        );

        let path = write_plist("com.local.watch", d);
        let job = parse_job(&path).expect("parse_job");

        // For WatchPaths jobs, schedule becomes the first watch path.
        assert_eq!(job.schedule, "/tmp/a");
        assert_eq!(
            job.watch_paths,
            vec!["/tmp/a".to_string(), "/tmp/b".to_string()]
        );
        assert_eq!(job.command, "/usr/bin/true");
        assert!(!job.is_login);
    }

    #[test]
    fn parse_job_login_run_at_load() {
        let _home = set_temp_home();

        let mut d = Dictionary::new();
        d.insert(
            "Label".to_string(),
            Value::String("com.local.login".to_string()),
        );
        d.insert("RunAtLoad".to_string(), Value::Boolean(true));
        d.insert(
            "ProgramArguments".to_string(),
            Value::Array(vec![Value::String("/bin/echo".to_string())]),
        );

        let path = write_plist("com.local.login", d);
        let job = parse_job(&path).expect("parse_job");

        assert_eq!(job.label, "com.local.login");
        assert_eq!(job.schedule, ON_LOGIN);
        assert_eq!(job.command, "/bin/echo");
        assert!(job.is_login);
    }

    #[test]
    fn list_plists_only_returns_plist_files() {
        let _home = set_temp_home();

        // One plist
        let mut d = Dictionary::new();
        d.insert(
            "Label".to_string(),
            Value::String("com.local.one".to_string()),
        );
        let _p = write_plist("com.local.one", d);

        // One non-plist file in same directory
        let dir = launch_agents_dir();
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("not_a_plist.txt"), "hello").expect("write");

        let plists = list_plists().expect("list_plists");
        assert!(plists.iter().any(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy() == "com.local.one.plist")
                .unwrap_or(false)
        }));
        assert!(!plists
            .iter()
            .any(|p| p.file_name().unwrap() == "not_a_plist.txt"));
    }
}
