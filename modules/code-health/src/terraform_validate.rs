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

/// What `tofu validate` says of the workspaces of a copy of Terraform:
/// each error, with each workspace it couldn't validate, and apart from
/// them each warning, such as an argument its provider or module has
/// marked deprecated.
#[derive(Debug, Default, PartialEq)]
pub struct Said {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// What `tofu validate` says of the workspaces of the copy of the
/// Terraform in `folder`.
pub fn invalid(folder: &Path, terraform: &[&File]) -> Result<Said, String> {
    let settings = PathBuf::from(format!("{}-tofurc", folder.display()));
    let none = PathBuf::from(format!("{}-providers", folder.display()));
    let mirror = env::var_os(PROVIDERS).map_or_else(|| none.clone(), PathBuf::from);
    let written = fs::create_dir_all(&none).and_then(|()| fs::write(&settings, config(&mirror)));
    let named = written
        .map_err(|error| format!("tofu's settings couldn't be written: {error}"))
        .and_then(|()| {
            let mut named = Said::default();
            for workspace in workspaces(terraform) {
                let said = validated(folder, workspace, terraform, &settings)?;
                named.errors.extend(said.errors);
                named.warnings.extend(said.warnings);
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
) -> Result<Said, String> {
    let shown = if workspace.is_empty() { "." } else { workspace };
    let unvalidated = |why: String| Said {
        errors: vec![format!("{shown}: {why}")],
        warnings: Vec::new(),
    };
    if reaches_elsewhere(workspace, terraform, &mut BTreeSet::new()) {
        return Ok(unvalidated(
            "uses a module from elsewhere, so it isn't validated".to_owned(),
        ));
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
        return Ok(unvalidated(format!("tofu init can't ready it: {why}")));
    }
    let validate = ["validate", "-json", "-no-color"];
    let checked = run_with("tofu", &validate, Some(&at), None, &setting)?;
    if checked.code > 1 {
        return Err(format!(
            "tofu validate ended {} in {shown}: {}",
            checked.code, checked.said
        ));
    }
    diagnostics(&checked.said, workspace)
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

/// Each error and each warning in `tofu validate -json`'s answer for
/// `workspace`, named at its file and line from the copy's top where it
/// has one.
fn diagnostics(said: &str, workspace: &str) -> Result<Said, String> {
    let validated: Validated = serde_json::from_str(said)
        .map_err(|error| format!("tofu validate's answer couldn't be read: {error}"))?;
    let mut named = Said::default();
    for Diagnostic {
        severity,
        summary,
        range,
    } in validated.diagnostics
    {
        let at = match (range, workspace) {
            (Some(Range { filename, start }), "") => format!("{filename}:{}", start.line),
            (Some(Range { filename, start }), _) => {
                format!("{workspace}/{filename}:{}", start.line)
            }
            (None, "") => ".".to_owned(),
            (None, _) => workspace.to_owned(),
        };
        match severity.as_str() {
            "error" => named.errors.push(format!("{at}: {summary}")),
            "warning" => named.warnings.push(format!("{at}: {summary}")),
            _ => {}
        }
    }
    Ok(named)
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
        let said = diagnostics(said, "ws").expect("tofu's answer wasn't read");
        assert_eq!(
            said.errors,
            [
                "ws/main.tf:2: Reference to undeclared input variable",
                "ws: Missing required provider",
            ],
            "an error was missed, misplaced or a warning taken for one"
        );
        assert_eq!(
            said.warnings,
            ["ws/main.tf:9: Deprecated attribute"],
            "a warning was missed, misplaced or an error taken for one"
        );
        let top = diagnostics(
            r#"{"diagnostics": [{"severity": "error", "summary": "S",
            "range": {"filename": "main.tf", "start": {"line": 2}}}]}"#,
            "",
        )
        .expect("tofu's answer wasn't read");
        assert_eq!(
            top.errors,
            ["main.tf:2: S"],
            "a top-level workspace's file was named with a folder"
        );
    }

    #[test]
    fn an_answer_tofu_garbled_is_refused() {
        let refused = diagnostics("{\"diagnostics\": 3}", "ws");
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
                "old/main.tf",
                "module \"m\" {\n  source = \"./sub\"\n  was    = 1\n}\n",
            ),
            file(
                "old/sub/main.tf",
                "variable \"was\" {\n  type       = number\n  deprecated = \"say is\"\n}\n",
            ),
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
            ],
            "what tofu refuses, or can't ready, was missed or misnamed"
        );
        assert_eq!(
            read.warned,
            ["old/main.tf:3: Variable marked as deprecated by the module author"],
            "a deprecated argument tofu warns of was missed or misnamed"
        );
    }
}
