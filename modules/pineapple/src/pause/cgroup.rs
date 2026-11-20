//! Whose a process's place in the user's manager is: Joel's own, or one
//! only agents and the services started for them run in, read from the
//! cgroup line the kernel gives it.

/// The slices of the user's manager that only agents' sessions and their
/// work run in (../../slices).
const AGENTS: [&str; 3] = ["agents.slice/", "jobs.slice/", "offload.slice/"];
/// The part of the jobs slice that holds Joel's own terminal's busy work.
const AWAITED: &str = "jobs.slice/jobs-awaited.slice/";

/// Why the place `cgroup` names is agents', or none when it may be
/// Joel's: a service the user's manager runs is started for something,
/// not typed, and agents' work and their shells run in slices of their
/// own; only the awaited part of the jobs slice holds Joel's own terminal
/// work. Outside the user's manager, as in a build's sandbox, it says
/// nothing.
#[must_use]
pub fn agents_cgroup(cgroup: &str) -> Option<String> {
    let path = cgroup
        .lines()
        .find_map(|line| line.strip_prefix("0::"))?
        .trim();
    let (_, inside) = path.split_once(".service/").filter(|(manager, _)| {
        manager
            .rsplit('/')
            .next()
            .is_some_and(|unit| unit.starts_with("user@"))
    })?;
    if inside.starts_with(AWAITED) {
        return None;
    }
    let leaf_is_a_service = inside
        .rsplit('/')
        .find(|part| !part.is_empty())
        .is_some_and(|leaf| leaf.ends_with(".service"));
    if AGENTS.iter().any(|slice| inside.starts_with(slice)) || leaf_is_a_service {
        return Some(format!(
            "it runs in {inside}, which the user's manager started for agents or a service"
        ));
    }
    None
}
