// src/core/manager.rs

use crate::core::constants::{INTERVALS, LABEL_NAMESPACE};
use crate::core::default_runtime::DefaultRuntime;
use crate::core::plist_io::{list_plists, parse_job};
use crate::core::runtime::Runtime;
use anyhow::{anyhow, Context, Result};
use plist::{Dictionary, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Derive a launchd label from a filesystem path.
///
/// Using the file stem provides a stable, human-friendly label component.
/// A fixed namespace prefix reduces the chance of collisions with other jobs.
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

/// Build EnvironmentVariables for the plist, if PATH is available.
///
/// launchd jobs do not inherit interactive shell initialization, so PATH may
/// be minimal. Persisting PATH helps handler commands that expect typical
/// user PATH contents, while keeping the environment surface area small.
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

/// Choose between "Program" and "ProgramArguments" plist styles.
///
/// launchd supports either:
/// - Program: a single executable path
/// - ProgramArguments: argv vector including the executable path and arguments
///
/// Prefer ProgramArguments when there is more than one token to preserve exact
/// argument boundaries without relying on shell parsing.
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

/// Split a crontab-like string into schedule and command tokens.
///
/// The expected input shape is:
/// "<minute> <hour> <day> <month> <weekday> <command...>"
///
/// This function intentionally uses whitespace splitting rather than shell parsing
/// because the CLI supports passing handler argv separately via "--", and because
/// launchd plist fields store argv tokens directly.
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

/// Convert a 5-field cron schedule into launchd's StartCalendarInterval dictionary.
///
/// The values are stored as strings to preserve "*" and numeric strings as provided.
/// launchd accepts strings for these keys in plist form and this keeps round-tripping
/// simple when showing jobs back to the user.
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

/// High-level interface for creating, listing, and removing LaunchAgents.
///
/// Runtime is injected to:
/// - Allow deterministic testing without touching the real filesystem or launchctl.
/// - Isolate platform-specific calls (uid, PATH lookup, command execution).
pub struct LaunchAgentManager<R: Runtime> {
    rt: R,
}

impl Default for LaunchAgentManager<DefaultRuntime> {
    fn default() -> Self {
        // DefaultRuntime is intentionally value-like (Copy) so construction is cheap
        // and does not require global initialization.
        Self { rt: DefaultRuntime }
    }
}

impl<R: Runtime> LaunchAgentManager<R> {
    /// Construct a manager with a caller-provided runtime.
    ///
    /// This is primarily used in tests to supply a FakeRuntime.
    pub fn new(rt: R) -> Self {
        Self { rt }
    }

    /// Print all jobs in a human-readable format.
    ///
    /// Jobs are derived from plist files found in the LaunchAgents directory.
    /// A job with an empty command is treated as disabled and printed with "@disabled".
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

    /// List job labels, optionally filtering by job type.
    ///
    /// filter_watchers_only limits output to WatchPaths jobs.
    /// filter_login_only limits output to RunAtLoad jobs.
    ///
    /// These filters exist to support separate CLI front-ends without duplicating
    /// parsing logic.
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

    /// Remove a job by label.
    ///
    /// Removal is best-effort for launchctl operations:
    /// - "bootout" and "remove" are attempted but failures are ignored to avoid
    ///   leaving the plist file undeleted due to transient launchctl state.
    /// - The plist file removal is treated as the authoritative removal step.
    pub fn remove(&self, label: &str) -> Result<()> {
        let plist_path = self.rt.launch_agents_dir().join(format!("{label}.plist"));
        if !plist_path.exists() {
            eprintln!("Not found; crontab or launch agent: {label}");
            return Ok(());
        }

        let uid = self.rt.uid();

        // Ignore errors because the job might not be loaded, might already be stopped,
        // or launchctl might return a non-zero status for benign reasons.
        self.rt
            .run_command(
                "launchctl",
                &["bootout".to_string(), format!("gui/{uid}/{label}")],
            )
            .ok();

        // "remove" is included as a second attempt to ensure the label is detached
        // from launchd even if bootout behavior differs across macOS versions.
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

    /// Supports a single string containing schedule + command.
    ///
    /// This keeps a compact interface for callers that already have a combined string,
    /// while delegating the more robust argv-based path to create_cron_parts.
    pub fn create_cron(&self, crontab: &str) -> Result<()> {
        let (schedule, command_parts) = split_schedule_and_command(crontab)?;
        self.create_cron_parts(&schedule, &command_parts)
    }

    /// Schedule and handler argv split.
    ///
    /// This method is intended for CLI usage where the handler is parsed by clap as
    /// a trailing argv list, preserving arguments that start with "-" and avoiding
    /// accidental option parsing.
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

    /// Create a WatchPaths-based LaunchAgent.
    ///
    /// WatchPaths jobs run when filesystem changes occur in the listed directories.
    /// Paths are validated as existing directories to fail fast and avoid creating
    /// inert jobs.
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

    /// Create a RunAtLoad (login) LaunchAgent.
    ///
    /// label_hint is used only for label derivation to provide a stable name that can
    /// be predicted by the caller, while allowing the handler program to be unrelated.
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

    /// Resolve and validate the handler executable, returning an argv list with an absolute path.
    ///
    /// The first argv element is treated as the executable. If it contains a path separator,
    /// it is treated as a direct path; otherwise it is resolved via PATH lookup.
    ///
    /// Returning an absolute path ensures the plist does not depend on launchd PATH behavior.
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

    /// Persist a plist to the LaunchAgents directory.
    ///
    /// Existing plist files are intentionally not overwritten to avoid clobbering
    /// user modifications or creating unexpected changes when re-running commands.
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

    /// Load the plist into launchd via launchctl bootstrap.
    ///
    /// bootstrap is used because it both loads and starts the job in the specified
    /// GUI domain, matching typical LaunchAgent behavior.
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
            // FakeRuntime records commands instead of running them, enabling assertions
            // about launchctl behavior without requiring macOS launchd in CI.
            Self {
                dir,
                commands: Arc::new(Mutex::new(vec![])),
                which_map: Arc::new(HashMap::new()),
            }
        }
    }

    impl Runtime for FakeRuntime {
        fn uid(&self) -> u32 {
            // A stable, typical macOS user id is used to make expected "gui/<uid>" strings
            // deterministic in tests.
            501
        }

        fn launch_agents_dir(&self) -> PathBuf {
            self.dir.clone()
        }

        fn which(&self, cmd: &str) -> Result<PathBuf> {
            // which_map allows tests to control resolution behavior precisely.
            self.which_map
                .get(cmd)
                .cloned()
                .ok_or_else(|| anyhow!("which failed for {cmd}"))
        }

        fn ensure_executable_file(&self, path: &Path) -> Result<()> {
            // For these unit tests, existence is sufficient to validate that resolution
            // happened and that the output plist points to a real file.
            if !path.exists() {
                return Err(anyhow!("not found"));
            }
            Ok(())
        }

        fn run_command(&self, program: &str, args: &[String]) -> Result<()> {
            // Record invocations so tests can assert that bootstrap and other operations
            // were attempted in the expected shape.
            self.commands
                .lock()
                .unwrap()
                .push((program.to_string(), args.to_vec()));
            Ok(())
        }
    }

    fn make_exe(td: &TempDir, name: &str) -> PathBuf {
        // Write a minimal script file and mark it executable on Unix so it passes
        // the manager's validation checks.
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
        // This test covers the critical path:
        // - A plist is generated and stored in the target LaunchAgents directory.
        // - launchctl bootstrap is invoked to load it.
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
