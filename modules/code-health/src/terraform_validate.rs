//! Terraform that `tofu` itself refuses: each workspace is readied with
//! `tofu init` from the providers this program was built with alone,
//! never a registry, and checked with `tofu validate`, which needs no
//! credentials and reaches nothing. A workspace using a module from
//! elsewhere can't be readied without fetching it, so it's named as not
//! validated rather than fetched; so is one no provider at hand allows.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::measure::File;
use crate::terraform::run_with;
use crate::terraform_docs::{Scan, workspaces};

/// Where the providers at hand are, as a tofu filesystem mirror; with it
/// unset, none are.
const PROVIDERS: &str = "CODE_HEALTH_TERRAFORM_PROVIDERS";

/// Each error `tofu validate` finds in the workspaces of the copy of the
/// Terraform in `folder`, and each workspace it couldn't validate.
pub fn invalid(folder: &Path, terraform: &[&File]) -> Result<Vec<String>, String> {
    let settings = PathBuf::from(format!("{}-tofurc", folder.display()));
    let none = PathBuf::from(format!("{}-providers", folder.display()));
    let mirror = env::var_os(PROVIDERS).map_or_else(|| none.clone(), PathBuf::from);
    let written = fs::create_dir_all(&none).and_then(|()| fs::write(&settings, config(&mirror)));
    let named = written
        .map_err(|error| format!("tofu's settings couldn't be written: {error}"))
        .and_then(|()| {
            let mut named = Vec::new();
            for workspace in workspaces(terraform) {
                named.extend(validated(folder, workspace, terraform, &settings)?);
            }
            Ok(named)
        });
    let _ = fs::remove_file(&settings);
    let _ = fs::remove_dir_all(&none);
    named
}

/// tofu's settings, listing only the mirror, so it asks no registry.
fn config(mirror: &Path) -> String {
    let path = mirror.display().to_string();
    format!("provider_installation {{\n  filesystem_mirror {{\n    path = {path:?}\n  }}\n}}\n")
}

/// What `tofu validate` finds in one workspace of the copy in `folder`,
/// or why it couldn't be validated.
fn validated(
    folder: &Path,
    workspace: &str,
    terraform: &[&File],
    settings: &Path,
) -> Result<Vec<String>, String> {
    let shown = if workspace.is_empty() { "." } else { workspace };
    if reaches_elsewhere(workspace, terraform, &mut BTreeSet::new()) {
        return Ok(vec![format!(
            "{shown}: uses a module from elsewhere, so it isn't validated"
        )]);
    }
    let at = folder.join(workspace);
    let setting = [("TF_CLI_CONFIG_FILE", settings)];
    let init = ["init", "-backend=false", "-input=false", "-no-color"];
    let readied = run_with("tofu", &init, Some(&at), None, &setting)?;
    if readied.code != 0 {
        let why = readied
            .said
            .lines()
            .find_map(|line| line.trim().strip_prefix("Error: "))
            .map_or_else(|| format!("it ended {}", readied.code), str::to_owned);
        return Ok(vec![format!("{shown}: tofu init can't ready it: {why}")]);
    }
    let validate = ["validate", "-json", "-no-color"];
    let checked = run_with("tofu", &validate, Some(&at), None, &setting)?;
    if checked.code > 1 {
        return Err(format!(
            "tofu validate ended {} in {shown}: {}",
            checked.code, checked.said
        ));
    }
    errors(&checked.said, workspace)
}

/// Whether a workspace, or a module it takes from a folder of the copy,
/// takes a module from elsewhere, which readying it would fetch.
fn reaches_elsewhere(workspace: &str, terraform: &[&File], seen: &mut BTreeSet<String>) -> bool {
    if !seen.insert(workspace.to_owned()) {
        return false;
    }
    let sources: Vec<String> = terraform
        .iter()
        .filter(|file| file.path.rsplit_once('/').map_or("", |(at, _)| at) == workspace)
        .flat_map(|file| sources(&file.text))
        .collect();
    sources.iter().any(|source| {
        local(workspace, source).is_none_or(|folder| reaches_elsewhere(&folder, terraform, seen))
    })
}

/// The folder of the copy a module source beside `workspace` names, or
/// `None` for one from elsewhere, above the copy's top among them.
fn local(workspace: &str, source: &str) -> Option<String> {
    if !source.starts_with("./") && !source.starts_with("../") {
        return None;
    }
    let mut parts: Vec<&str> = workspace
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    for part in source.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            folder => parts.push(folder),
        }
    }
    Some(parts.join("/"))
}

/// Where each module block in `text` takes its module from.
fn sources(text: &str) -> Vec<String> {
    let mut scan = Scan::default();
    let mut module = false;
    let mut found = Vec::new();
    for line in text.lines() {
        let depth = scan.depth;
        // The scan blanks strings, so the source is read from the line.
        let code = scan.code(line);
        let raw = line.trim();
        if depth == 0 {
            module = code.trim_start().starts_with("module");
        } else if depth == 1
            && module
            && code.trim_start().starts_with("source")
            && let Some((_, value)) = raw.split_once('=')
            && let Some(source) = value.trim().strip_prefix('"')
        {
            found.push(source.split('"').next().unwrap_or(source).to_owned());
        }
    }
    found
}

/// `tofu validate -json`'s answer, as far as it's read.
#[derive(Deserialize)]
struct Validated {
    #[serde(default)]
    diagnostics: Vec<Diagnostic>,
}

#[derive(Deserialize)]
struct Diagnostic {
    severity: String,
    summary: String,
    range: Option<Range>,
}

#[derive(Deserialize)]
struct Range {
    filename: String,
    start: Start,
}

#[derive(Deserialize)]
struct Start {
    line: u64,
}

/// Each error in `tofu validate -json`'s answer for `workspace`, named
/// at its file and line from the copy's top where it has one.
fn errors(said: &str, workspace: &str) -> Result<Vec<String>, String> {
    let validated: Validated = serde_json::from_str(said)
        .map_err(|error| format!("tofu validate's answer couldn't be read: {error}"))?;
    let named = validated
        .diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == "error")
        .map(|Diagnostic { summary, range, .. }| {
            let at = match (range, workspace) {
                (Some(Range { filename, start }), "") => format!("{filename}:{}", start.line),
                (Some(Range { filename, start }), _) => {
                    format!("{workspace}/{filename}:{}", start.line)
                }
                (None, "") => ".".to_owned(),
                (None, _) => workspace.to_owned(),
            };
            format!("{at}: {summary}")
        });
    Ok(named.collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::file;
    use crate::terraform::read;

    #[test]
    fn each_error_is_named_at_its_workspaces_file_and_line() {
        let said = r#"{"valid": false, "error_count": 2, "diagnostics": [
            {"severity": "error", "summary": "Reference to undeclared input variable",
             "range": {"filename": "main.tf", "start": {"line": 2, "column": 11}}},
            {"severity": "warning", "summary": "Deprecated attribute",
             "range": {"filename": "main.tf", "start": {"line": 9, "column": 3}}},
            {"severity": "error", "summary": "Missing required provider"}
        ]}"#;
        assert_eq!(
            errors(said, "ws").expect("tofu's answer wasn't read"),
            [
                "ws/main.tf:2: Reference to undeclared input variable",
                "ws: Missing required provider",
            ]
        );
        let top = errors(said, "").expect("tofu's answer wasn't read");
        assert_eq!(top[0], "main.tf:2: Reference to undeclared input variable");
    }

    #[test]
    fn an_answer_tofu_garbled_is_refused() {
        let refused = errors("{\"diagnostics\": 3}", "ws");
        assert!(refused.is_err(), "a garbled answer read as {refused:?}");
    }

    #[test]
    fn what_tofu_refuses_is_named_and_what_it_cant_ready_is_said() {
        let files = [
            file(
                "ws/main.tf",
                "module \"m\" {\n  source = \"./sub\"\n}\n\noutput \"o\" {\n  value = var.missing\n}\n",
            ),
            file("ws/sub/main.tf", "output \"x\" {\n  value = 1\n}\n"),
            file(
                "far/main.tf",
                "module \"m\" {\n  source = \"git::https://example.invalid/m.git\"\n}\n",
            ),
            file(
                "needs/main.tf",
                concat!(
                    "terraform {\n  required_providers {\n    google = {\n",
                    "      source = \"hashicorp/google\"\n    }\n  }\n}\n",
                ),
            ),
            file("fine/main.tf", "output \"o\" {\n  value = 1\n}\n"),
            file(
                "near/main.tf",
                "module \"m\" {\n  source = \"../shared/far\"\n}\n",
            ),
            file(
                "shared/far/main.tf",
                "module \"m\" {\n  source = \"git::https://example.invalid/m.git\"\n}\n",
            ),
        ];
        let read = read(&files).expect("Terraform was read as none");
        let read = read.expect("the tools couldn't read it");
        assert_eq!(
            read.invalid,
            [
                "far: uses a module from elsewhere, so it isn't validated",
                "near: uses a module from elsewhere, so it isn't validated",
                "needs: tofu init can't ready it: Failed to query available provider packages",
                "shared/far: uses a module from elsewhere, so it isn't validated",
                "ws/main.tf:6: Reference to undeclared input variable",
            ]
        );
    }
}
