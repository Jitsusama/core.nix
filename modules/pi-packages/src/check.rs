//! Whether a built version is whole, asked before it is swapped in.
//!
//! Three things have gone wrong with an install that nothing caught
//! until a session loaded it: a dependency missing, a native module's
//! build skipped (pnpm 10 runs only the builds a package allows, and on
//! 26 October sqlite3 arrived without its binding), and a binding that
//! is there but will not load. So a version must have every dependency
//! its manifest names, no native module without a binding, and every
//! native module must load in a node of its own.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// Where a native module's install puts what it built or fetched.
const BINDING_DIRS: [&str; 2] = ["build", "prebuilds"];

/// What is wrong with a version, each said so a person can act on it,
/// or nothing when it is whole.
///
/// # Errors
///
/// Says why the version could not be read at all.
pub fn problems(
    version: &Path,
    node: &Path,
    env: &[(String, String)],
) -> Result<Vec<String>, String> {
    let mut found = missing_dependencies(version)?;
    for module in native_modules(version) {
        let name = module_name(&module);
        if !has_binding(&module) {
            found.push(format!(
                "{name}'s native build did not run, so it has no binding"
            ));
            continue;
        }
        if let Err(why) = loads(version, &module, node, env) {
            found.push(format!("{name} does not load: {why}"));
        }
    }
    Ok(found)
}

/// The dependencies the manifest names that `node_modules` lacks.
fn missing_dependencies(version: &Path) -> Result<Vec<String>, String> {
    let manifest = version.join("package.json");
    let Ok(text) = fs::read_to_string(&manifest) else {
        // No manifest: a package of files alone, with nothing to install.
        return Ok(Vec::new());
    };
    let value: Value =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", manifest.display()))?;
    let Some(dependencies) = value["dependencies"].as_object() else {
        return Ok(Vec::new());
    };
    Ok(dependencies
        .keys()
        .filter(|name| !version.join("node_modules").join(name).exists())
        .map(|name| format!("{name} is not installed"))
        .collect())
}

/// Every package the version reaches with a native part to build, which
/// is what a `binding.gyp` says, each at its real path. They are found by
/// following the version's links, not by reading a folder of its own:
/// pnpm's global virtual store keeps every package outside the version,
/// with each one's own dependencies linked beside it, and a look in the
/// version's `.pnpm` would find none and pass a build that never ran.
fn native_modules(version: &Path) -> Vec<PathBuf> {
    let mut seen = BTreeSet::new();
    let mut looked = BTreeSet::new();
    let mut waiting = vec![version.join("node_modules")];
    let mut found = Vec::new();
    while let Some(modules) = waiting.pop() {
        if !looked.insert(modules.clone()) {
            continue;
        }
        for package in packages_in(&modules) {
            let Ok(real) = fs::canonicalize(&package) else {
                // A link to nothing is a dependency the missing check names.
                continue;
            };
            if !seen.insert(real.clone()) {
                continue;
            }
            if real.join("binding.gyp").is_file() {
                found.push(real.clone());
            }
            if let Some(beside) = holder_of(&real) {
                waiting.push(beside);
            }
            waiting.push(real.join("node_modules"));
        }
    }
    found.sort();
    found
}

/// The `node_modules` folder a package sits in, where pnpm links its own
/// dependencies beside it, a scoped package's one folder further up.
fn holder_of(package: &Path) -> Option<PathBuf> {
    let parent = package.parent()?;
    let scoped = parent
        .file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with('@'));
    let holder = if scoped { parent.parent()? } else { parent };
    (holder.file_name()? == "node_modules").then(|| holder.to_path_buf())
}

/// The package folders and links directly in a `node_modules`, scoped
/// ones included.
fn packages_in(modules: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(modules) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('@') {
            found.extend(packages_in(&path));
        } else if !name.starts_with('.') {
            found.push(path);
        }
    }
    found
}

/// A package's name, as its folder in `node_modules` says it.
fn module_name(package: &Path) -> String {
    let mut parts: Vec<String> = package
        .components()
        .rev()
        .take(2)
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    parts.reverse();
    match parts.as_slice() {
        [scope, name] if scope.starts_with('@') => format!("{scope}/{name}"),
        [.., name] => name.clone(),
        [] => package.display().to_string(),
    }
}

/// Whether a native module's folder holds a `.node` binding.
fn has_binding(package: &Path) -> bool {
    BINDING_DIRS
        .iter()
        .any(|dir| holds_binding(&package.join(dir)))
}

fn holds_binding(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            holds_binding(&path)
        } else {
            path.extension()
                .is_some_and(|extension| extension == "node")
        }
    })
}

/// Load a native module in a node of its own, from the version.
fn loads(
    version: &Path,
    module: &Path,
    node: &Path,
    env: &[(String, String)],
) -> Result<(), String> {
    let output = Command::new(node)
        .args(["-e", "require(process.argv[1])"])
        .arg(module)
        .current_dir(version)
        .env_clear()
        .envs(env.iter().map(|(key, value)| (key, value)))
        .output()
        .map_err(|error| format!("{}: {error}", node.display()))?;
    if output.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(&output.stderr);
    Err(said
        .lines()
        .find(|line| line.contains("Error"))
        .unwrap_or_else(|| said.lines().next().unwrap_or("it exited with a failure"))
        .trim()
        .to_owned())
}
