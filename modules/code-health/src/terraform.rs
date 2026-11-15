//! Terraform held as other code is, as the quality standard asks: the
//! files `tofu fmt` would write otherwise, and what tflint's own rules
//! find in each workspace, its recommended set, as a workspace with no
//! config of its own gets. Both run only where a repository holds
//! Terraform, so one holding none has no figure rather than a nought.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use crate::measure::{File, extension};

/// What a repository's Terraform was found to hold: each file named with
/// why `tofu fmt` would write it otherwise, and each tflint finding.
#[derive(Debug, Default, PartialEq)]
pub struct Read {
    pub unformatted: Vec<String>,
    pub findings: Vec<String>,
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
    let findings = linted(terraform)?;
    Ok(Read {
        unformatted,
        findings,
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

/// What tflint finds in every workspace of a copy of the Terraform, made
/// in a folder of its own and removed once read.
fn linted(terraform: &[&File]) -> Result<Vec<String>, String> {
    let folder = scratch();
    let found = copied(&folder, terraform).and_then(|()| {
        run(
            "tflint",
            &["--recursive", "--format=compact", "--no-color", "--force"],
            Some(&folder),
            None,
        )
    });
    let _ = fs::remove_dir_all(&folder);
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
struct Ran {
    code: i32,
    said: String,
}

fn run(
    program: &str,
    arguments: &[&str],
    folder: Option<&Path>,
    input: Option<&[u8]>,
) -> Result<Ran, String> {
    let mut command = Command::new(program);
    command
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
}
