//! The machine's units a stop pauses: which of the agents' timers run and
//! which units of agents' work there are, and freezing or thawing them so
//! that a unit gone meanwhile counts as done.

use super::Plan;
use super::system::Runs;

/// The slice agents' work runs in, and the slice beside it for their
/// timed work. Joel's own terminal's work runs in another, the awaited
/// slice, which is never paused (../slices/jobs-sorter).
const JOBS: &str = "jobs.slice";
const TIMED: &str = "jobs-timed.slice";
/// The prefix of the slices agents' work is sorted into, by quest.
const WORK: &str = "jobs-work";
/// Where agents put a push that waits on its walk slot themselves, which
/// the slices' rules count as agents' too (../slices/agents-ask-first.rules).
const SHARED: &str = "shared.slice";

/// A unit as systemd shows it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shown {
    /// Its name.
    pub id: String,
    /// The slice it runs in.
    pub slice: String,
    /// Whether it's frozen, thawing, or running.
    pub freezer: String,
    /// Whether it's active.
    pub active: String,
}

/// The plan's timers running now, by name, so a resume starts only those.
///
/// # Errors
///
/// Why systemd couldn't list them.
pub fn active_timers(plan: &Plan, runs: &mut dyn Runs) -> Result<Vec<String>, String> {
    if plan.timers.is_empty() {
        return Ok(Vec::new());
    }
    let args = user(
        &[
            "list-units",
            "--type=timer",
            "--state=active",
            "--plain",
            "--no-legend",
            "--no-pager",
        ],
        &plan.timers,
    );
    runs.run(&plan.systemctl, &args)
        .map(|listed| first_words(&listed))
}

/// Each unit of agents' work running now: every service and scope in the
/// jobs slice itself, in its timed slice, in each slice of the work it
/// sorts by quest, and in the shared slice where pushes wait, but never
/// one the plan keeps, wherever it runs.
///
/// # Errors
///
/// Why systemd couldn't list them.
pub fn agents_work(plan: &Plan, runs: &mut dyn Runs) -> Result<Vec<Shown>, String> {
    let listed = user(
        &[
            "list-units",
            "--type=service,scope",
            "--state=running",
            "--plain",
            "--no-legend",
            "--no-pager",
        ],
        &[],
    );
    let units = runs
        .run(&plan.systemctl, &listed)
        .map(|text| first_words(&text))?;
    if units.is_empty() {
        return Ok(Vec::new());
    }
    Ok(show(plan, runs, &units)?
        .into_iter()
        .filter(|unit| {
            [JOBS, TIMED, SHARED].contains(&unit.slice.as_str()) || unit.slice.starts_with(WORK)
        })
        .filter(|unit| !plan.keep.iter().any(|kept| matches(kept, &unit.id)))
        .collect())
}

/// Each of `units` as systemd shows it.
///
/// # Errors
///
/// Why systemd couldn't show them.
pub fn show(plan: &Plan, runs: &mut dyn Runs, units: &[String]) -> Result<Vec<Shown>, String> {
    let args = user(
        &["show", "--property=Id,Slice,FreezerState,ActiveState"],
        units,
    );
    runs.run(&plan.systemctl, &args).map(|text| shown(&text))
}

/// Freezes or thaws `units`, `verb` as systemd says it, and answers why
/// for each that is still running and still not as asked: one that ended
/// meanwhile is as good as frozen or thawed, so it isn't a failure.
pub fn settle(plan: &Plan, runs: &mut dyn Runs, verb: &str, units: &[String]) -> Vec<String> {
    if units.is_empty() {
        return Vec::new();
    }
    if runs.run(&plan.systemctl, &user(&[verb], units)).is_ok() {
        return Vec::new();
    }
    let wanted = if verb == "freeze" {
        "frozen"
    } else {
        "running"
    };
    match show(plan, runs, units) {
        Ok(now) => now
            .into_iter()
            .filter(|unit| unit.active == "active" && unit.freezer != wanted)
            .map(|unit| unit.id)
            .collect(),
        // Not knowing which are left, every one is.
        Err(_) => units.to_vec(),
    }
}

/// Whether `name` is `pattern`, where each `*` stands for any run of
/// characters.
pub fn matches(pattern: &str, name: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = name.strip_prefix(first) else {
        return false;
    };
    let parts: Vec<&str> = parts.collect();
    let Some((last, middle)) = parts.split_last() else {
        return rest.is_empty();
    };
    for part in middle {
        let Some(at) = rest.find(part) else {
            return false;
        };
        rest = &rest[at + part.len()..];
    }
    rest.ends_with(last)
}

/// Each unit `systemctl show` describes, from its blocks of lines parted
/// by a blank one, read in whatever order systemd says their keys: the
/// systemd here says `Id=` first, which nothing promises.
fn shown(text: &str) -> Vec<Shown> {
    text.split("\n\n")
        .filter_map(|block| {
            let mut unit = Shown::default();
            for line in block.lines() {
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                let field = match key {
                    "Id" => &mut unit.id,
                    "Slice" => &mut unit.slice,
                    "FreezerState" => &mut unit.freezer,
                    "ActiveState" => &mut unit.active,
                    _ => continue,
                };
                value.clone_into(field);
            }
            (!unit.id.is_empty()).then_some(unit)
        })
        .collect()
}

fn first_words(listed: &str) -> Vec<String> {
    listed
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

/// systemctl's words for the user's own manager: `act`, then `names`.
pub fn user(act: &[&str], names: &[String]) -> Vec<String> {
    let mut args = vec!["--user".to_owned()];
    args.extend(act.iter().map(|word| (*word).to_owned()));
    args.extend(names.iter().cloned());
    args
}
