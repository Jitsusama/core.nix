//! Terraform held as other code is, as the quality standard asks: the
//! files `tofu fmt` would write otherwise, what tflint finds in each
//! workspace held to all its own rules and Google's (`RULES`), and what
//! trivy's checks find that would leave what
//! it makes open to attack, from the checks it was built with, so it
//! fetches nothing, and what `tofu validate` refuses in each workspace
//! (`terraform_validate.rs`). Each runs only where a repository holds Terraform,
//! so one holding none has no figure rather than a nought.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use serde::Deserialize;

use crate::measure::{File, extension};
use crate::terraform_validate;

/// What a repository's Terraform was found to hold: each file named with
/// why `tofu fmt` would write it otherwise, each tflint finding, each of
/// trivy's security findings, and each error `tofu validate` finds.
#[derive(Debug, Default, PartialEq)]
pub struct Read {
    pub unformatted: Vec<String>,
    pub findings: Vec<String>,
    pub insecure: Vec<String>,
    pub invalid: Vec<String>,
}

/// Whether a path is Terraform: a `.tf` file or a `.tfvars` one.
pub fn is_terraform(path: &str) -> bool {
    matches!(extension(path), "tf" | "tfvars")
}

/// What a repository's Terraform holds, or `None` where it holds none.
pub fn read(files: &[File]) -> Option<Result<Read, String>> {
    let terraform: Vec<&File> = files
        .iter()
        .filter(|file| is_terraform(&file.path))
        .collect();
    if terraform.is_empty() {
        return None;
    }
    Some(read_all(&terraform))
}

fn read_all(terraform: &[&File]) -> Result<Read, String> {
    let mut unformatted = Vec::new();
    for file in terraform {
        if let Some(why) = formatting(file)? {
            unformatted.push(format!("{}: {why}", file.path));
        }
    }
    let folder = scratch();
    // Validated last, since readying a workspace writes into the copy.
    let read = copied(&folder, terraform).and_then(|()| {
        let findings = linted(&folder)?;
        let insecure = scanned(&folder)?;
        Ok((
            findings,
            insecure,
            terraform_validate::invalid(&folder, terraform)?,
        ))
    });
    let _ = fs::remove_dir_all(&folder);
    let (findings, insecure, invalid) = read?;
    Ok(Read {
        unformatted,
        findings,
        insecure,
        invalid,
    })
}

/// Why `tofu fmt` would write a file otherwise, or `None` where it
/// wouldn't: it reads the text on its input, so nothing is written.
fn formatting(file: &File) -> Result<Option<&'static str>, String> {
    let checked = run(
        "tofu",
        &["fmt", "-check", "-no-color", "-"],
        None,
        Some(file.text.as_bytes()),
    )?;
    match checked.code {
        0 => Ok(None),
        3 => Ok(Some("not as tofu fmt writes it")),
        2 => Ok(Some("tofu fmt can't parse it")),
        code => Err(format!(
            "tofu fmt ended {code} on {}: {}",
            file.path, checked.said
        )),
    }
}

/// The rules every workspace is held to, in place of any it sets itself,
/// so each is measured against one standard: all of tflint's own and all
/// of Google's, but for the two `terraform_docs.rs` already counts. Local
/// modules are linted where they lie rather than through what calls them.
const RULES: &str = r#"config {
  call_module_type = "none"
}
plugin "terraform" {
  enabled = true
  preset  = "all"
}
plugin "google" {
  enabled = true
}
rule "terraform_documented_variables" {
  enabled = false
}
rule "terraform_documented_outputs" {
  enabled = false
}
"#;

/// What tflint finds in every workspace of the copy of the Terraform in
/// `folder`, held to `RULES`, written beside it and removed once read.
fn linted(folder: &Path) -> Result<Vec<String>, String> {
    let rules = PathBuf::from(format!("{}-tflint.hcl", folder.display()));
    fs::write(&rules, RULES)
        .map_err(|error| format!("tflint's rules couldn't be written: {error}"))?;
    let config = format!("--config={}", rules.display());
    let found = run(
        "tflint",
        &[
            "--recursive",
            &config,
            "--format=compact",
            "--no-color",
            "--force",
        ],
        Some(folder),
        None,
    );
    let _ = fs::remove_file(&rules);
    let found = found?;
    let mark = folder
        .file_name()
        .map(|name| format!("{}/", name.to_string_lossy()))
        .unwrap_or_default();
    let findings = findings(&found.said, &mark);
    // tflint ends 1 when a workspace couldn't be loaded, which is named.
    let named_failure = findings.iter().any(|finding| finding.ends_with(LOAD));
    if found.code == 0 || (found.code == 1 && named_failure) {
        Ok(findings)
    } else {
        Err(format!("tflint ended {}: {}", found.code, found.said))
    }
}

/// What trivy's checks find in the copy of the Terraform in `folder`,
/// with a cache beside it, removed once read, so nothing is kept.
fn scanned(folder: &Path) -> Result<Vec<String>, String> {
    let cache = PathBuf::from(format!("{}-cache", folder.display()));
    let cache_dir = cache.to_string_lossy().into_owned();
    let found = run(
        "trivy",
        &[
            "config",
            "--skip-check-update",
            "--skip-version-check",
            "--quiet",
            "--format=json",
            "--cache-dir",
            &cache_dir,
            ".",
        ],
        Some(folder),
        None,
    );
    let _ = fs::remove_dir_all(&cache);
    let found = found?;
    if found.code != 0 {
        return Err(format!("trivy ended {}: {}", found.code, found.said));
    }
    insecure(&found.said)
}

/// trivy's JSON answer, as far as it's read: each file's checks.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Scan {
    #[serde(default)]
    results: Vec<Scanned>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Scanned {
    target: String,
    #[serde(default)]
    misconfigurations: Vec<Check>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Check {
    #[serde(rename = "ID")]
    id: String,
    severity: String,
    status: String,
    title: String,
    #[serde(default)]
    cause_metadata: Cause,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Cause {
    start_line: Option<u64>,
}

/// Each failed check in trivy's JSON answer, named at its file and line.
fn insecure(said: &str) -> Result<Vec<String>, String> {
    let scan: Scan = serde_json::from_str(said)
        .map_err(|error| format!("trivy's answer couldn't be read: {error}"))?;
    let named = scan.results.iter().flat_map(|scanned| {
        scanned
            .misconfigurations
            .iter()
            .filter(|check| check.status == "FAIL")
            .map(|check| {
                let at = check
                    .cause_metadata
                    .start_line
                    .map_or(String::new(), |line| format!(":{line}"));
                let Check {
                    id,
                    severity,
                    title,
                    ..
                } = check;
                format!("{}{at}: {severity} {id} - {title}", scanned.target)
            })
    });
    Ok(named.collect())
}

/// What a workspace tflint couldn't load is named with.
const LOAD: &str = "tflint can't load it";

/// Each finding in tflint's compact answer, its path from the copy's top
/// (tflint says it from elsewhere, so it's taken past `mark`, the copy's
/// own folder), and each workspace it couldn't load.
fn findings(said: &str, mark: &str) -> Vec<String> {
    said.lines()
        .filter_map(|line| {
            if let Some(rest) = line.strip_prefix("Failed to run in ") {
                let workspace = rest.split(';').next().unwrap_or(rest);
                return Some(format!("{workspace}: {LOAD}"));
            }
            let level = [": Error - ", ": Warning - ", ": Notice - "]
                .iter()
                .any(|level| line.contains(level));
            if !level {
                return None;
            }
            let at = line.find(mark).map_or(0, |at| at + mark.len());
            Some(line[at..].to_owned())
        })
        .collect()
}

/// A folder of its own for one copy, so reads at once don't share one.
fn scratch() -> PathBuf {
    static MADE: AtomicUsize = AtomicUsize::new(0);
    let made = MADE.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!(
        "code-health-terraform-{}-{made}",
        std::process::id()
    ))
}

/// The Terraform written under `folder` at its own paths.
fn copied(folder: &Path, terraform: &[&File]) -> Result<(), String> {
    let _ = fs::remove_dir_all(folder);
    for file in terraform {
        let path = folder.join(&file.path);
        let written = path
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::write(&path, &file.text));
        written.map_err(|error| format!("{} couldn't be copied: {error}", file.path))?;
    }
    Ok(())
}

/// How a program ended, and all it said, its errors after its output.
pub struct Ran {
    pub code: i32,
    pub said: String,
}

fn run(
    program: &str,
    arguments: &[&str],
    folder: Option<&Path>,
    input: Option<&[u8]>,
) -> Result<Ran, String> {
    run_with(program, arguments, folder, input, &[])
}

/// A program run as `run` does, with `settings` added to its environment.
pub fn run_with(
    program: &str,
    arguments: &[&str],
    folder: Option<&Path>,
    input: Option<&[u8]>,
    settings: &[(&str, &Path)],
) -> Result<Ran, String> {
    let mut command = Command::new(program);
    command
        .envs(settings.iter().copied())
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(folder) = folder {
        command.current_dir(folder);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("{program} couldn't start: {error}"))?;
    let stdin = child.stdin.take();
    let input = input.unwrap_or_default().to_vec();
    // Written aside, so a program answering as it reads never waits on us.
    let writer = thread::spawn(move || stdin.map(|mut stdin| stdin.write_all(&input)));
    let output = child
        .wait_with_output()
        .map_err(|error| format!("{program} couldn't be read: {error}"))?;
    let _ = writer.join();
    let mut said = String::from_utf8_lossy(&output.stdout).into_owned();
    said.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(Ran {
        code: output.status.code().unwrap_or(-1),
        said,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::file;

    #[test]
    fn a_repository_with_no_terraform_has_no_reading() {
        let files = [file("main.rs", "fn main() {}\n")];
        assert!(read(&files).is_none(), "no Terraform was read as some");
    }

    #[test]
    fn terraform_is_its_tf_and_tfvars_files() {
        let terraform: Vec<bool> = ["a/main.tf", "prod.tfvars", ".tflint.hcl", "tf.md"]
            .into_iter()
            .map(is_terraform)
            .collect();
        assert_eq!(terraform, [true, true, false, false]);
    }

    #[test]
    fn each_finding_is_named_from_the_copys_top() {
        let said = concat!(
            "2 issue(s) found:\n\n",
            "../../tmp/code-health-terraform-7-0/ws/main.tf:27:3: Warning - ",
            "local.zone is declared but not used (terraform_unused_declarations)\n",
            "../../tmp/code-health-terraform-7-0/ws/a.tf:1:1: Error - bad (rule)\n",
            "Failed to run in broken; exit status 1\n\n",
            "Failed to load the root module; x.tf:1,14-15: Unclosed block\n",
        );
        assert_eq!(
            findings(said, "code-health-terraform-7-0/"),
            [
                "ws/main.tf:27:3: Warning - local.zone is declared but not used \
                 (terraform_unused_declarations)",
                "ws/a.tf:1:1: Error - bad (rule)",
                "broken: tflint can't load it",
            ]
        );
    }

    #[test]
    fn each_failed_check_is_named_at_its_file_and_line() {
        let said = r#"{"Results": [
            {"Target": "ws", "Class": "config"},
            {"Target": "ws/compute.tf", "Misconfigurations": [
                {"ID": "GCP-0030", "Severity": "MEDIUM", "Status": "FAIL",
                 "Title": "Disable project-wide SSH keys for all instances",
                 "CauseMetadata": {"StartLine": 57}},
                {"ID": "GCP-0041", "Severity": "LOW", "Status": "PASS",
                 "Title": "Passed", "CauseMetadata": {"StartLine": 19}},
                {"ID": "GCP-0033", "Severity": "LOW", "Status": "FAIL",
                 "Title": "Encrypt disks", "CauseMetadata": {}}
            ]}
        ]}"#;
        assert_eq!(
            insecure(said).expect("trivy's answer wasn't read"),
            [
                "ws/compute.tf:57: MEDIUM GCP-0030 - \
                 Disable project-wide SSH keys for all instances",
                "ws/compute.tf: LOW GCP-0033 - Encrypt disks",
            ]
        );
    }

    #[test]
    fn an_answer_trivy_garbled_is_refused() {
        let refused = insecure("{\"Results\": 3}");
        assert!(refused.is_err(), "a garbled answer read as {refused:?}");
    }

    #[test]
    fn each_read_has_a_folder_of_its_own() {
        assert_ne!(scratch(), scratch(), "two reads shared one folder");
    }

    #[test]
    fn a_files_formatting_and_its_workspaces_findings_are_named() {
        let files = [
            file("ws/main.tf", "locals {\n  zone = \"a\"\n}\n"),
            file("ws/vars.tf", "variable \"v\" {\ntype=string\n}\n"),
            file("broken/x.tf", "resource \"a\" {\n"),
        ];
        let read = read(&files).expect("Terraform was read as none");
        let read = read.expect("the tools couldn't read it");
        assert_eq!(
            read.unformatted,
            [
                "ws/vars.tf: not as tofu fmt writes it",
                "broken/x.tf: tofu fmt can't parse it",
            ]
        );
        assert!(
            read.findings
                .contains(&"broken: tflint can't load it".to_owned()),
            "the broken workspace went unnamed: {:?}",
            read.findings
        );
        assert!(
            read.findings.iter().any(|finding| finding
                .starts_with("ws/main.tf:2:3: Warning - local.zone is declared but not used")),
            "the unused local went unnamed: {:?}",
            read.findings
        );
    }

    #[test]
    fn a_workspace_is_held_to_every_terraform_rule() {
        let files = [file("ws/main.tf", "output \"o\" {\n  value = 1\n}\n")];
        let read = read(&files).expect("Terraform was read as none");
        let read = read.expect("the tools couldn't read it");
        assert!(
            read.findings
                .iter()
                .any(|finding| finding.ends_with("(terraform_standard_module_structure)")),
            "the output outside outputs.tf went unnamed: {:?}",
            read.findings
        );
    }

    #[test]
    fn a_workspace_is_held_to_googles_rules() {
        let files = [file(
            "ws/compute.tf",
            concat!(
                "resource \"google_compute_instance\" \"vm\" {\n",
                "  machine_type = \"n1-nonexistent\"\n",
                "}\n",
            ),
        )];
        let read = read(&files).expect("Terraform was read as none");
        let read = read.expect("the tools couldn't read it");
        assert!(
            read.findings.iter().any(|finding| {
                finding.ends_with("(google_compute_instance_invalid_machine_type)")
            }),
            "the machine type Google has none of went unnamed: {:?}",
            read.findings
        );
    }

    #[test]
    fn a_workspaces_security_findings_are_named() {
        let files = [file(
            "ws/compute.tf",
            concat!(
                "resource \"google_compute_instance\" \"vm\" {\n",
                "  name         = \"vm\"\n",
                "  machine_type = \"e2-small\"\n",
                "  zone         = \"us-central1-a\"\n",
                "}\n",
            ),
        )];
        let read = read(&files).expect("Terraform was read as none");
        let read = read.expect("the tools couldn't read it");
        assert!(
            read.insecure
                .iter()
                .any(|finding| finding.starts_with("ws/compute.tf:1: MEDIUM GCP-0067 - ")),
            "the instance without secure boot went unnamed: {:?}",
            read.insecure
        );
    }
}
