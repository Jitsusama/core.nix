//! Terraform that doesn't say what it is, as the quality standard asks
//! of all code: each variable and output with no description, which is
//! what its readers and `tofu` itself show of it, and each workspace (a
//! folder holding Terraform) with no README beside it to say what it
//! makes and how it's applied. It reads text only, so it runs no tool.

use std::collections::BTreeSet;

use crate::measure::{File, extension};

/// Each variable and output with no description, and each workspace with
/// no README, or `None` where a repository holds no Terraform.
pub fn undocumented(files: &[File]) -> Option<Vec<String>> {
    let terraform: Vec<&File> = files
        .iter()
        .filter(|file| matches!(extension(&file.path), "tf" | "tfvars"))
        .collect();
    if terraform.is_empty() {
        return None;
    }
    let mut named: Vec<String> = terraform
        .iter()
        .flat_map(|file| undescribed(&file.text).map(move |block| format!("{}:{block}", file.path)))
        .collect();
    let noted: BTreeSet<&str> = files.iter().map(|file| file.path.as_str()).collect();
    let workspaces: BTreeSet<&str> = terraform
        .iter()
        .filter(|file| extension(&file.path) == "tf")
        .map(|file| file.path.rsplit_once('/').map_or("", |(folder, _)| folder))
        .collect();
    for workspace in workspaces {
        let readme = if workspace.is_empty() {
            "README.md".to_owned()
        } else {
            format!("{workspace}/README.md")
        };
        if !noted.contains(readme.as_str()) {
            let shown = if workspace.is_empty() { "." } else { workspace };
            named.push(format!("{shown}: workspace has no README"));
        }
    }
    Some(named)
}

/// Each variable and output block in `text` with no description of its
/// own, as its line and what it is: only an attribute of the block itself
/// counts, never one in a block within it, a comment or a heredoc.
fn undescribed(text: &str) -> impl Iterator<Item = String> {
    let mut scan = Scan::default();
    let mut open: Option<(usize, String, bool)> = None;
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let depth = scan.depth;
        let code = scan.code(line);
        let trimmed = code.trim_start();
        // The scan blanks strings, so the block's name is read from the line.
        if depth == 0
            && let Some(block) = block(line.trim_start())
        {
            let inline = trimmed
                .split_once('{')
                .is_some_and(|(_, rest)| sets_description(rest));
            open = Some((index + 1, block, inline));
        } else if depth == 1
            && sets_description(trimmed)
            && let Some((_, _, described)) = open.as_mut()
        {
            *described = true;
        }
        if scan.depth == 0
            && let Some((line, block, described)) = open.take()
            && !described
        {
            found.push(format!("{line}: {block} has no description"));
        }
    }
    found.into_iter()
}

/// The block a top-level line opens, as `variable "name"` or
/// `output "name"`, or `None` for any other.
fn block(line: &str) -> Option<String> {
    let (kind, rest) = line.split_once(char::is_whitespace)?;
    if !matches!(kind, "variable" | "output") {
        return None;
    }
    let name = rest.trim_start().strip_prefix('"')?.split('"').next()?;
    Some(format!("{kind} \"{name}\""))
}

/// Whether code opens with a `description` attribute.
fn sets_description(code: &str) -> bool {
    code.trim_start()
        .strip_prefix("description")
        .is_some_and(|rest| {
            rest.trim_start().starts_with('=') && !rest.trim_start().starts_with("==")
        })
}

/// Where a scan stands between lines: how deep in braces, and whether in
/// a block comment or a heredoc, whose end marker it holds.
#[derive(Default)]
struct Scan {
    depth: usize,
    comment: bool,
    heredoc: Option<String>,
}

/// What a scan is in within a line: a string, or an interpolation in one
/// counting its own braces.
enum Within {
    Text,
    Interpolation(usize),
}

impl Scan {
    /// A line's code with its comments, strings and heredoc text blanked,
    /// moving the depth by its braces.
    fn code(&mut self, line: &str) -> String {
        if let Some(marker) = &self.heredoc {
            if line.trim() == marker {
                self.heredoc = None;
            }
            return String::new();
        }
        let chars: Vec<char> = line.chars().collect();
        let mut code = String::new();
        let mut within: Vec<Within> = Vec::new();
        let mut at = 0;
        while at < chars.len() {
            let here = chars[at];
            let next = chars.get(at + 1).copied();
            if self.comment {
                if here == '*' && next == Some('/') {
                    self.comment = false;
                    at += 1;
                }
            } else if let Some(Within::Text) = within.last() {
                match (here, next) {
                    ('\\', _) => at += 1,
                    ('"', _) => {
                        within.pop();
                    }
                    ('$' | '%', Some('{')) => {
                        within.push(Within::Interpolation(0));
                        at += 1;
                    }
                    _ => {}
                }
            } else {
                match (here, next) {
                    ('#', _) | ('/', Some('/')) => break,
                    ('/', Some('*')) => {
                        self.comment = true;
                        at += 1;
                    }
                    ('"', _) => within.push(Within::Text),
                    ('<', Some('<')) => {
                        let marker: String = chars[at + 2..].iter().collect();
                        let marker = marker.trim_start_matches('-').trim();
                        if !marker.is_empty()
                            && marker.chars().all(|c| c.is_alphanumeric() || c == '_')
                        {
                            self.heredoc = Some(marker.to_owned());
                            code.push_str("<<");
                            break;
                        }
                        code.push(here);
                    }
                    ('{', _) => {
                        match within.last_mut() {
                            Some(Within::Interpolation(braces)) => *braces += 1,
                            _ => self.depth += 1,
                        }
                        code.push(here);
                    }
                    ('}', _) => {
                        match within.last_mut() {
                            Some(Within::Interpolation(0)) => {
                                within.pop();
                            }
                            Some(Within::Interpolation(braces)) => *braces -= 1,
                            _ => self.depth = self.depth.saturating_sub(1),
                        }
                        code.push(here);
                    }
                    _ => code.push(here),
                }
            }
            at += 1;
        }
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::file;

    #[test]
    fn a_repository_with_no_terraform_has_no_reading() {
        let files = [file("README.md", "# A\n")];
        assert!(undocumented(&files).is_none(), "no Terraform read as some");
    }

    #[test]
    fn each_variable_and_output_without_a_description_is_named() {
        let files = [
            file("ws/README.md", "# The workspace\n"),
            file(
                "ws/vars.tf",
                concat!(
                    "variable \"zone\" {\n",
                    "  type        = string\n",
                    "  description = \"Where the VM runs.\"\n",
                    "}\n",
                    "\n",
                    "variable \"size\" {\n",
                    "  type = string\n",
                    "  validation {\n",
                    "    condition     = var.size != \"\"\n",
                    "    error_message = \"description = is not this\"\n",
                    "  }\n",
                    "}\n",
                    "variable \"empty\" {}\n",
                    "output \"ip\" { description = \"The VM's address.\" }\n",
                    "output \"name\" {\n",
                    "  value = \"${var.zone}-{x}\"\n",
                    "}\n",
                ),
            ),
        ];
        assert_eq!(
            undocumented(&files).expect("Terraform read as none"),
            [
                "ws/vars.tf:6: variable \"size\" has no description",
                "ws/vars.tf:13: variable \"empty\" has no description",
                "ws/vars.tf:15: output \"name\" has no description",
            ]
        );
    }

    #[test]
    fn a_description_inside_a_nested_block_or_a_heredoc_doesnt_count() {
        let files = [
            file("README.md", "# All of it\n"),
            file(
                "main.tf",
                concat!(
                    "variable \"rules\" {\n",
                    "  default = <<-EOT\n",
                    "    description = \"not an attribute\"\n",
                    "    }\n",
                    "  EOT\n",
                    "  # description = \"a comment\"\n",
                    "  /* description = \"a comment\" */\n",
                    "  type = object({ description = string })\n",
                    "}\n",
                    "variable \"after\" {\n",
                    "  description = \"Still read at the right depth.\"\n",
                    "}\n",
                ),
            ),
        ];
        assert_eq!(
            undocumented(&files).expect("Terraform read as none"),
            ["main.tf:1: variable \"rules\" has no description"]
        );
    }

    #[test]
    fn each_workspace_without_a_readme_is_named() {
        let files = [
            file("a/main.tf", "locals {}\n"),
            file("a/README.md", "# A\n"),
            file("b/main.tf", "locals {}\n"),
            file("b/c/main.tf", "locals {}\n"),
            file("b/c/prod.tfvars", "x = 1\n"),
            file("d/prod.tfvars", "x = 1\n"),
            file("top.tf", "locals {}\n"),
        ];
        assert_eq!(
            undocumented(&files).expect("Terraform read as none"),
            [
                ".: workspace has no README",
                "b: workspace has no README",
                "b/c: workspace has no README",
            ]
        );
    }
}
