// tests/cli.rs

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;

fn temp_home() -> TempDir {
    // Many tests rely on LaunchAgents directory behavior, which is derived from HOME.
    // Using a TempDir allows isolating side effects and prevents writing into a real user profile.
    let td = TempDir::new().expect("tempdir");
    td
}

fn set_home(cmd: &mut Command, td: &TempDir) {
    // Set HOME for the child process so the binaries read and write LaunchAgents under TempDir.
    // This avoids requiring a particular machine setup and keeps the tests hermetic.
    cmd.env("HOME", td.path());
}

#[test]
fn cronl_help_works() {
    // Smoke test that the binary is built and clap can render help output.
    // This catches argument schema regressions early.
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("cronl"));
    cmd.arg("--help");
    cmd.assert().success();
}

#[test]
fn watchl_help_works() {
    // Equivalent smoke test for watchl to ensure its CLI wiring stays valid.
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("watchl"));
    cmd.arg("--help");
    cmd.assert().success();
}

#[test]
fn loginl_help_works() {
    // Equivalent smoke test for loginl to ensure its CLI wiring stays valid.
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("loginl"));
    cmd.arg("--help");
    cmd.assert().success();
}

#[test]
fn cronl_list_works_with_empty_home() {
    // Listing should succeed when LaunchAgents directory does not exist yet.
    // This verifies that list_plists treats missing directories as empty rather than erroring.
    let td = temp_home();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("cronl"));
    set_home(&mut cmd, &td);

    cmd.arg("--list");
    cmd.assert().success();
}

#[test]
fn watchl_list_works_with_empty_home() {
    // Same behavior is expected across binaries; each should tolerate a fresh HOME.
    let td = temp_home();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("watchl"));
    set_home(&mut cmd, &td);

    cmd.arg("--list");
    cmd.assert().success();
}

#[test]
fn loginl_list_works_with_empty_home() {
    // Same behavior is expected across binaries; each should tolerate a fresh HOME.
    let td = temp_home();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("loginl"));
    set_home(&mut cmd, &td);

    cmd.arg("--list");
    cmd.assert().success();
}

#[test]
fn cronl_show_all_works_with_empty_home() {
    let td = temp_home();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("cronl"));
    set_home(&mut cmd, &td);

    cmd.arg("--show-all");
    cmd.assert().success();
}

#[test]
fn watchl_show_all_works_with_empty_home() {
    // Consistency check: show-all should not require LaunchAgents directory to exist.
    let td = temp_home();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("watchl"));
    set_home(&mut cmd, &td);

    cmd.arg("--show-all");
    cmd.assert().success();
}

#[test]
fn loginl_show_all_works_with_empty_home() {
    // Consistency check: show-all should not require LaunchAgents directory to exist.
    let td = temp_home();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("loginl"));
    set_home(&mut cmd, &td);

    cmd.arg("--show-all");
    cmd.assert().success();
}

#[test]
fn remove_nonexistent_plist_does_not_call_launchctl_and_exits_ok() {
    let td = temp_home();

    // cronl --remove label (no plist exists -> should early-return success)
    //
    // This is important for usability: removal should be idempotent and should not
    // attempt launchctl operations when there is no corresponding plist file.
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("cronl"));
    set_home(&mut cmd, &td);

    cmd.args(["--remove", "com.local.does-not-exist"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Not found"));
}

#[test]
fn show_all_prints_job_when_plist_exists() {
    use plist::{Dictionary, Value};
    use std::fs;

    let td = temp_home();
    let agents_dir = td.path().join("Library").join("LaunchAgents");
    fs::create_dir_all(&agents_dir).expect("create LaunchAgents dir");

    // Create a single job plist
    //
    // Writing a plist directly exercises the parsing and printing path of the binary
    // without requiring any launchctl side effects.
    let label = "com.local.one";

    let mut intervals = Dictionary::new();
    intervals.insert("Minute".to_string(), Value::String("0".to_string()));
    intervals.insert("Hour".to_string(), Value::String("1".to_string()));
    intervals.insert("Day".to_string(), Value::String("*".to_string()));
    intervals.insert("Month".to_string(), Value::String("*".to_string()));
    intervals.insert("Weekday".to_string(), Value::String("*".to_string()));

    let mut d = Dictionary::new();
    d.insert("Label".to_string(), Value::String(label.to_string()));
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

    let path = agents_dir.join(format!("{label}.plist"));
    plist::to_file_xml(&path, &Value::Dictionary(d)).expect("write plist");

    // Now run cronl --show-all and assert it prints the schedule + command
    //
    // The output format is designed to be human-readable and stable for simple scripting.
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("cronl"));
    set_home(&mut cmd, &td);

    cmd.arg("--show-all");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("0 1 * * * /bin/echo hello"));
}
