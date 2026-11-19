//! Which subtest a line of a VM test's log says failed, as pytest says
//! it. Apart from the rest of `--changed`, since it changes only with how
//! pytest and Nix write a log, and together with its copies: merge-net's
//! `failed_on` in subtests.rs, changed-checks' in quarantine.rs and
//! run-subtests.py's.

/// The subtest a line of a VM test's log says failed, as pytest says it:
/// `FAILED vm-pytest-x/x-vm-test.py::test_y - why`, or `ERROR` for one
/// that erred in a fixture, a parametrized case by its function's name.
/// Only pytest's own line counts, past what Nix puts before it, so a
/// guest's echoed `FAILED` names none.
pub fn failed_on(line: &str) -> Option<&str> {
    let said = pytests_own(line);
    let rest = said
        .strip_prefix("FAILED ")
        .or_else(|| said.strip_prefix("ERROR "))?;
    let (_, name) = rest.split_whitespace().next()?.rsplit_once("::")?;
    let name = name.split('[').next().unwrap_or(name);
    (name.starts_with("test_") && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .then_some(name)
}

/// What pytest wrote on one line of a build's log, past what Nix puts
/// before it: `vm-test-run-x> ` on a line, or spaces then `> ` on those
/// that follow it.
fn pytests_own(line: &str) -> &str {
    if let Some(rest) = line.trim_start().strip_prefix("> ") {
        return rest;
    }
    match line.split_once("> ") {
        Some((build, rest)) if !build.is_empty() && !build.contains(' ') => rest,
        _ => line,
    }
}
