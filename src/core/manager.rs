// src/core/manager.rs

use crate::core::constants::{INTERVALS, LABEL_NAMESPACE};
use crate::core::default_runtime::DefaultRuntime;
use crate::core::plist_io::{list_plists, parse_job};
use crate::core::runtime::Runtime;
use anyhow::{anyhow, Context, Result};
use plist::{Dictionary, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn derive_label_from_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut parts: Vec<&str> = LABEL_NAMESPACE.to_vec();
    parts.push(&stem);
    parts.join(".")
}

fn build_env_vars() -> Option<Value> {
    if let Ok(p) = std::env::var("PATH") {
        if !p.trim().is_empty() {
            let mut d = Dictionary::new();
            d.insert("PATH".to_string(), Value::String(p));
            return Some(Value::Dictionary(d));
        }
    }
    None
}

fn program_or_args(argv: &[String]) -> (String, Value) {
    if argv.len() == 1 {
        ("Program".to_string(), Value::String(argv[0].clone()))
    } else {
        (
            "ProgramArguments".to_string(),
            Value::Array(argv.iter().cloned().map(Value::String).collect()),
        )
    }
}

fn split_schedule_and_command(crontab: &str) -> Result<(String, Vec<String>)> {
    let parts: Vec<String> = crontab
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    if parts.len() < INTERVALS.len() + 1 {
        return Err(anyhow!(
            "Invalid crontab string: expected {} schedule parts plus a command",
            INTERVALS.len()
        ));
    }

    let schedule = parts[..INTERVALS.len()].join(" ");
    let command_parts = parts[INTERVALS.len()..].to_vec();
    Ok((schedule, command_parts))
}

fn parse_calendar_interval(schedule: &str) -> Result<Value> {
    let mut interval_dict = Dictionary::new();
    let fields: Vec<&str> = schedule.split_whitespace().collect();

    if fields.len() != INTERVALS.len() {
        return Err(anyhow!(
            "Invalid cron schedule: expected {} parts, got {}",
            INTERVALS.len(),
            fields.len()
        ));
    }

    for (k, v) in INTERVALS.iter().zip(fields.iter()) {
        interval_dict.insert((*k).to_string(), Value::String((*v).to_string()));
    }

    Ok(Value::Dictionary(interval_dict))
}

pub struct LaunchAgentManager<R: Runtime> {
    rt: R,
}

impl Default for LaunchAgentManager<DefaultRuntime> {
    fn default() -> Self {
        Self { rt: DefaultRuntime }
    }
}

impl<R: Runtime> LaunchAgentManager<R> {
    pub fn new(rt: R) -> Self {
        Self { rt }
    }

    pub fn show_all(&self) -> Result<()> {
        for p in list_plists()? {
            let job = parse_job(&p)?;
            if job.command.trim().is_empty() {
                println!("@disabled {}", p.display());
            } else {
                println!("{} {}", job.schedule, job.command);
            }
        }
        Ok(())
    }

    pub fn list_labels(&self, filter_watchers_only: bool, filter_login_only: bool) -> Result<()> {
        for p in list_plists()? {
            let job = parse_job(&p)?;
            if filter_watchers_only && job.watch_paths.is_empty() {
                continue;
            }
            if filter_login_only && !job.is_login {
                continue;
            }
            println!("{}", job.label);
        }
        Ok(())
    }

    pub fn remove(&self, label: &str) -> Result<()> {
        let plist_path = self.rt.launch_agents_dir().join(format!("{label}.plist"));
        if !plist_path.exists() {
            eprintln!("Not found; crontab or launch agent: {label}");
            return Ok(());
        }

        let uid = self.rt.uid();

        self.rt
            .run_command(
                "launchctl",
                &["bootout".to_string(), format!("gui/{uid}/{label}")],
            )
            .ok();

        self.rt
            .run_command(
                "launchctl",
                &["remove".to_string(), format!("gui/{uid}/{label}")],
            )
            .ok();

        fs::remove_file(&plist_path)
            .with_context(|| format!("Failed to remove plist: {}", plist_path.display()))?;

        println!("Removed launch agent: {label}");
        Ok(())
    }

    /// Existing API: a single string containing schedule + command.
    pub fn create_cron(&self, crontab: &str) -> Result<()> {
        let (schedule, command_parts) = split_schedule_and_command(crontab)?;
        self.create_cron_parts(&schedule, &command_parts)
    }

    /// New API: schedule and handler argv split (easy to test and matches `--` CLI).
    pub fn create_cron_parts(&self, schedule: &str, handler_argv: &[String]) -> Result<()> {
        let argv = self.extract_program_path(handler_argv)?;

        let exe_path = PathBuf::from(&argv[0]);
        let label = derive_label_from_path(&exe_path);

        let mut root = Dictionary::new();
        root.insert("Label".to_string(), Value::String(label.clone()));
        root.insert(
            "StartCalendarInterval".to_string(),
            parse_calendar_interval(schedule)?,
        );

        if let Some(env) = build_env_vars() {
            root.insert("EnvironmentVariables".to_string(), env);
        }

        let (k, v) = program_or_args(&argv);
        root.insert(k, v);

        let plist_path = self.save_plist(&label, Value::Dictionary(root))?;
        self.bootstrap(&plist_path)?;
        println!("Created and enabled launchd job: {label}");
        Ok(())
    }

    pub fn create_watch(&self, watch_paths: &[PathBuf], handler_argv: &[String]) -> Result<()> {
        if watch_paths.is_empty() {
            return Err(anyhow!("A watch path is required"));
        }
        for p in watch_paths {
            if !p.exists() || !p.is_dir() {
                return Err(anyhow!("Directory not found: {}", p.display()));
            }
        }

        let argv = self.extract_program_path(handler_argv)?;
        let label = derive_label_from_path(&watch_paths[0]);

        let mut root = Dictionary::new();
        root.insert("Label".to_string(), Value::String(label.clone()));
        root.insert(
            "WatchPaths".to_string(),
            Value::Array(
                watch_paths
                    .iter()
                    .map(|p| Value::String(p.to_string_lossy().to_string()))
                    .collect(),
            ),
        );

        if let Some(env) = build_env_vars() {
            root.insert("EnvironmentVariables".to_string(), env);
        }

        let (k, v) = program_or_args(&argv);
        root.insert(k, v);

        let plist_path = self.save_plist(&label, Value::Dictionary(root))?;
        self.bootstrap(&plist_path)?;
        println!("Created and enabled launchd job: {label}");
        Ok(())
    }

    pub fn create_login(&self, label_hint: &Path, handler_argv: &[String]) -> Result<()> {
        let argv = self.extract_program_path(handler_argv)?;
        let label = derive_label_from_path(label_hint);

        let mut root = Dictionary::new();
        root.insert("Label".to_string(), Value::String(label.clone()));
        root.insert("RunAtLoad".to_string(), Value::Boolean(true));

        if let Some(env) = build_env_vars() {
            root.insert("EnvironmentVariables".to_string(), env);
        }

        let (k, v) = program_or_args(&argv);
        root.insert(k, v);

        let plist_path = self.save_plist(&label, Value::Dictionary(root))?;
        self.bootstrap(&plist_path)?;
        println!("Created and enabled launchd job: {label}");
        Ok(())
    }

    fn extract_program_path(&self, argv: &[String]) -> Result<Vec<String>> {
        if argv.is_empty() {
            return Err(anyhow!("Executable is required"));
        }

        let first = &argv[0];
        let exe_path = if first.contains(std::path::MAIN_SEPARATOR) {
            PathBuf::from(first)
        } else {
            self.rt.which(first)?
        };

        self.rt.ensure_executable_file(&exe_path)?;

        let mut out = vec![exe_path.to_string_lossy().to_string()];
        out.extend_from_slice(&argv[1..]);
        Ok(out)
    }

    fn save_plist(&self, label: &str, value: Value) -> Result<PathBuf> {
        let dir = self.rt.launch_agents_dir();
        fs::create_dir_all(&dir).with_context(|| format!("Failed to create {}", dir.display()))?;

        let path = dir.join(format!("{label}.plist"));
        if path.exists() {
            return Ok(path);
        }

        plist::to_file_xml(&path, &value)
            .with_context(|| format!("Failed to write plist: {}", path.display()))?;
        Ok(path)
    }

    fn bootstrap(&self, plist_path: &Path) -> Result<()> {
        let uid = self.rt.uid();
        self.rt.run_command(
            "launchctl",
            &[
                "bootstrap".to_string(),
                format!("gui/{uid}"),
                plist_path.to_string_lossy().to_string(),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::LaunchAgentManager;
    use crate::core::runtime::Runtime;
    use anyhow::{anyhow, Result};
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    #[derive(Clone)]
    struct FakeRuntime {
        dir: PathBuf,
        commands: Arc<Mutex<Vec<(String, Vec<String>)>>>,
        which_map: Arc<HashMap<String, PathBuf>>,
    }

    impl FakeRuntime {
        fn new(dir: PathBuf) -> Self {
            Self {
                dir,
                commands: Arc::new(Mutex::new(vec![])),
                which_map: Arc::new(HashMap::new()),
            }
        }
    }

    impl Runtime for FakeRuntime {
        fn uid(&self) -> u32 {
            501
        }

        fn launch_agents_dir(&self) -> PathBuf {
            self.dir.clone()
        }

        fn which(&self, cmd: &str) -> Result<PathBuf> {
            self.which_map
                .get(cmd)
                .cloned()
                .ok_or_else(|| anyhow!("which failed for {cmd}"))
        }

        fn ensure_executable_file(&self, path: &Path) -> Result<()> {
            if !path.exists() {
                return Err(anyhow!("not found"));
            }
            Ok(())
        }

        fn run_command(&self, program: &str, args: &[String]) -> Result<()> {
            self.commands
                .lock()
                .unwrap()
                .push((program.to_string(), args.to_vec()));
            Ok(())
        }
    }

    fn make_exe(td: &TempDir, name: &str) -> PathBuf {
        let p = td.path().join(name);
        fs::write(&p, "#!/bin/sh\necho ok\n").expect("write exe");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&p).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&p, perms).unwrap();
        }
        p
    }

    #[test]
    fn create_cron_writes_plist_and_bootstraps() {
        let td = TempDir::new().unwrap();
        let agents_dir = td.path().join("Library").join("LaunchAgents");
        let exe = make_exe(&td, "archive.rb");

        let rt = FakeRuntime::new(agents_dir.clone());
        let mgr = LaunchAgentManager::new(rt.clone());

        let schedule = "0 1 * * *";
        let handler = vec![exe.to_string_lossy().to_string(), "/tmp/out".to_string()];

        mgr.create_cron_parts(schedule, &handler).unwrap();

        // plist exists
        let label = "com.local.archive";
        let plist_path = agents_dir.join(format!("{label}.plist"));
        assert!(plist_path.exists());

        // launchctl bootstrap called
        let cmds = rt.commands.lock().unwrap().clone();
        assert!(cmds.iter().any(|(p, a)| {
            p == "launchctl"
                && a.get(0).map(|s| s.as_str()) == Some("bootstrap")
                && a.iter().any(|x| x.contains("gui/"))
        }));
    }
}
