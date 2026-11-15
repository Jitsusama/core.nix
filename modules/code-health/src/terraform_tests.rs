//! Terraform that nothing tests, as the quality standard asks of all
//! code: each workspace (a folder holding Terraform) with no test file
//! `tofu test` would run, one ending `.tftest.hcl` or `.tofutest.hcl`
//! in the folder itself or in its `tests` folder, where `tofu test`
//! looks. It reads paths only, so it runs no tool.

use std::collections::BTreeSet;

use crate::measure::extension;

/// Each workspace with no test of its own, or `None` where a repository
/// holds no Terraform.
pub fn untested(paths: &[String]) -> Option<Vec<String>> {
    let folder = |path: &'_ str| path.rsplit_once('/').map_or("", |(at, _)| at).to_owned();
    let workspaces: BTreeSet<String> = paths
        .iter()
        .filter(|path| extension(path) == "tf")
        .map(|path| folder(path))
        .filter(|at| !at.split('/').any(|part| part == "tests"))
        .collect();
    if workspaces.is_empty() {
        return None;
    }
    let tested: BTreeSet<String> = paths
        .iter()
        .filter(|path| extension(path) == "hcl" && is_test(path))
        .map(|path| folder(path))
        .map(|at| match at.strip_suffix("/tests") {
            Some(workspace) => workspace.to_owned(),
            None if at == "tests" => String::new(),
            None => at,
        })
        .collect();
    Some(
        workspaces
            .difference(&tested)
            .map(|at| {
                let shown = if at.is_empty() { "." } else { at };
                format!("{shown}: workspace has no tofu test")
            })
            .collect(),
    )
}

/// Whether an `.hcl` file is one `tofu test` runs, by its name's end.
fn is_test(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    name.ends_with(".tftest.hcl") || name.ends_with(".tofutest.hcl")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(each: &[&str]) -> Vec<String> {
        each.iter().map(|&path| path.to_owned()).collect()
    }

    #[test]
    fn a_repository_with_no_terraform_has_no_reading() {
        assert!(
            untested(&paths(&["README.md", "src/main.rs"])).is_none(),
            "no Terraform read as some"
        );
    }

    #[test]
    fn each_workspace_with_no_test_is_named() {
        let read = untested(&paths(&[
            "main.tf",
            "beside/main.tf",
            "beside/main.tftest.hcl",
            "folder/main.tf",
            "folder/tests/plan.tofutest.hcl",
            "folder/tests/setup/main.tf",
            "bare/main.tf",
            "bare/variables.tf",
            "bare/README.md",
            "near/main.tf",
            "near/far/main.tftest.hcl",
            "vars/only.tfvars",
        ]));
        assert_eq!(
            read.expect("Terraform read as none"),
            [
                ".: workspace has no tofu test",
                "bare: workspace has no tofu test",
                "near: workspace has no tofu test"
            ],
            "a workspace is tested by a test file beside it or in its tests folder, a helper \
             module within its tests folder is part of its tests, and a test deeper down \
             tests only its own folder",
        );
    }

    #[test]
    fn a_test_file_alone_is_no_terraform() {
        let read = untested(&paths(&["ws/tests/main.tftest.hcl"]));
        assert!(read.is_none(), "a test file alone isn't Terraform to read");
    }
}
