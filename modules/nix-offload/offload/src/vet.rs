//! Whether a lent builder's instance is one root may trust, from what the
//! cloud says of it rather than from its name.
//!
//! The person's account makes each lent builder and names it to `lines`,
//! and anything running as that account can change the instance too: give
//! it another startup script, an image of its own, or ssh keys Google's
//! guest agent makes sudoers of. Its host key, which only the instance can
//! publish, then vouches for whatever it became, and root takes back what
//! it builds. So an instance passes only while the cloud says it runs the
//! machine's own startup script, from an image root allows, carrying no
//! metadata but what its maker sets, with root's own key as the hook's,
//! and hasn't started again since root first trusted it: a reset is how a
//! changed startup script or disk would come to run.
//!
//! What an instance is now doesn't say what it ran before: one booted with
//! a script of its own could write to its disk as root, then be set back
//! and reset. So at first trust the audit log must say nothing was done to
//! it or its boot disk but its making, its last start must be its first
//! boot, and the log is read again once its slower entries are in. From
//! then on its fingerprints must stay as they were, since any change
//! moves them, even one undone between two looks. The project's own
//! metadata, which every instance takes where it sets nothing itself, may
//! hold only what each builder's own overrides.

use serde_json::Value;

/// The metadata a lent builder may carry: what its maker sets, none of
/// which makes an account or runs anything but its startup script.
const METADATA: [&str; 7] = [
    "block-project-ssh-keys",
    "enable-guest-attributes",
    "enable-oslogin",
    "offload-hook-key",
    "offload-max-jobs",
    "offload-there-key",
    "startup-script",
];

/// The project's metadata a lent builder may sit under: each overridden
/// by the instance's own `block-project-ssh-keys` and `enable-oslogin`.
const PROJECT_METADATA: [&str; 2] = ["enable-oslogin", "ssh-keys"];

/// The call that makes an instance, as the audit log names it.
const MAKING: &str = ".compute.instances.insert";

/// What root holds a lent builder to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pinned {
    /// The startup script, whole, that the machine's own config names.
    pub script: String,
    /// Root's public key, as its `.pub` file holds it.
    pub hook_key: String,
    /// What a boot disk's source image may start with.
    pub images: Vec<String>,
}

/// When the instance `described`, as the cloud describes it in JSON, last
/// started, when it's one root may trust; why not otherwise.
pub fn vetted(described: &Value, pinned: &Pinned) -> Result<String, String> {
    let status = described["status"].as_str().unwrap_or_default();
    if status != "RUNNING" {
        return Err(format!("is {status}, not running"));
    }
    let items = described["metadata"]["items"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let value = |key: &str| {
        items
            .iter()
            .find(|item| item["key"] == key)
            .and_then(|item| item["value"].as_str())
    };
    if let Some(other) = items
        .iter()
        .filter_map(|item| item["key"].as_str())
        .find(|key| !METADATA.contains(key))
    {
        return Err(format!(
            "carries the metadata {other}, which no builder is made with"
        ));
    }
    if !value("block-project-ssh-keys").is_some_and(|said| said.eq_ignore_ascii_case("true")) {
        return Err("takes the project's ssh keys".to_owned());
    }
    if value("enable-oslogin").is_some_and(|said| said.eq_ignore_ascii_case("true")) {
        return Err("lets OS Login make accounts".to_owned());
    }
    if !value("offload-hook-key").is_some_and(|said| same_key(said, &pinned.hook_key)) {
        return Err("names another key than root's for its hook".to_owned());
    }
    if value("startup-script") != Some(pinned.script.as_str()) {
        return Err("runs another startup script than this machine's".to_owned());
    }
    described["lastStartTimestamp"]
        .as_str()
        .filter(|started| !started.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "says nothing of when it started".to_owned())
}

/// How long after its making an instance may first start. Its first start
/// is the end of the insert that makes it, which took six seconds on 10
/// November and 2 minutes 26 seconds twice on 11 November, when root
/// refused both builders for it. Ten minutes, as long as `lines` waits to
/// read the audit log again, costs nothing: a stop and start, or a reset,
/// within it still shows in the log, read at first trust and at the reread.
const BOOTS_WITHIN: i64 = 600;

/// Whether the instance `described` last started at its first boot, soon
/// after it was made; why not otherwise. Asked at first trust, it catches
/// a stop and start, or a reset, the audit log hasn't shown yet.
pub fn started_once(described: &Value) -> Result<(), String> {
    let at = |field: &str| described[field].as_str().unwrap_or_default();
    let (made, started) = (at("creationTimestamp"), at("lastStartTimestamp"));
    match (seconds(made), seconds(started)) {
        (Some(was), Some(is)) if (0..=BOOTS_WITHIN).contains(&(is - was)) => Ok(()),
        (Some(_), Some(_)) => Err(format!(
            "last started at {started}, long after it was made at {made}, so it was started \
             again"
        )),
        _ => Err("says nothing clear of when it was made and started".to_owned()),
    }
}

/// The seconds since 1970 that a cloud timestamp such as
/// `2026-11-10T19:32:10.212-08:00` names, its fraction dropped.
fn seconds(stamp: &str) -> Option<i64> {
    let number = |text: &str| text.parse::<i64>().ok();
    let (date, rest) = stamp.split_once('T')?;
    let mut day = date.splitn(3, '-').map(number);
    let (year, month, day) = (day.next()??, day.next()??, day.next()??);
    let time = rest.get(..8)?;
    let mut clock = time.splitn(3, ':').map(number);
    let (hour, minute, second) = (clock.next()??, clock.next()??, clock.next()??);
    let zone = rest
        .get(8..)?
        .trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let offset = if zone == "Z" {
        0
    } else {
        let sign = match zone.get(..1)? {
            "+" => 1,
            "-" => -1,
            _ => return None,
        };
        let (hours, minutes) = zone.get(1..)?.split_once(':')?;
        sign * (number(hours)? * 3600 + number(minutes)? * 60)
    };
    // Days since 1970 by Howard Hinnant's civil-from-days, run backwards.
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let of_era_days = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    let days = era * 146_097 + of_era_days - 719_468;
    Some(days * 86_400 + hour * 3600 + minute * 60 + second - offset)
}

/// The instance's fingerprint and its metadata's, as `described`, which
/// the cloud moves with every change to either.
pub fn marks(described: &Value) -> Option<String> {
    let instance = described["fingerprint"]
        .as_str()
        .filter(|mark| !mark.is_empty())?;
    let metadata = described["metadata"]["fingerprint"]
        .as_str()
        .filter(|mark| !mark.is_empty())?;
    Some(format!("{instance} {metadata}"))
}

/// Whether the project `described` sets no metadata a builder would take
/// but what its own overrides; why not otherwise.
pub fn project_holds(described: &Value) -> Result<(), String> {
    let items = described["commonInstanceMetadata"]["items"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    match items
        .iter()
        .filter_map(|item| item["key"].as_str())
        .find(|key| !PROJECT_METADATA.contains(key))
    {
        Some(other) => Err(format!(
            "sits in a project setting the metadata {other}, which every builder would take"
        )),
        None => Ok(()),
    }
}

/// Whether the audit log's `entries` for an instance and its boot disk
/// hold its making alone; why not otherwise. A making is two entries, its
/// operation's first and last, and the disk made with it has none.
pub fn made_alone(entries: &Value) -> Result<(), String> {
    let entries = entries
        .as_array()
        .ok_or("has an audit log that couldn't be read")?;
    let mut made = false;
    for entry in entries {
        let said = &entry["protoPayload"];
        let method = said["methodName"].as_str().unwrap_or_default();
        let resource = said["resourceName"].as_str().unwrap_or_default();
        if method.ends_with(MAKING) && resource.contains("/instances/") {
            made = true;
        } else {
            let at = entry["timestamp"].as_str().unwrap_or("an unknown time");
            return Err(format!("was changed once made, by {method} at {at}"));
        }
    }
    if made {
        Ok(())
    } else {
        Err("isn't in the audit log as made yet".to_owned())
    }
}

/// Whether two public keys, each `TYPE KEY [COMMENT]`, are the same key.
fn same_key(one: &str, other: &str) -> bool {
    fn key(text: &str) -> Vec<&str> {
        text.split_whitespace().take(2).collect()
    }
    let one = key(one);
    one.len() == 2 && one == key(other)
}

/// The name of the instance's boot disk, as `described` names it.
pub fn boot_disk(described: &Value) -> Option<String> {
    described["disks"]
        .as_array()?
        .iter()
        .find(|disk| disk["boot"] == true)?["source"]
        .as_str()?
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

/// Whether the disk `described` was made from an image `pinned` allows;
/// why not otherwise.
pub fn imaged(described: &Value, pinned: &Pinned) -> Result<(), String> {
    let image = described["sourceImage"].as_str().unwrap_or_default();
    if !image.is_empty()
        && pinned
            .images
            .iter()
            .any(|allowed| image.starts_with(allowed))
    {
        Ok(())
    } else {
        Err(format!(
            "boots from {image:?}, which isn't an image root allows"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ROOT: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIRoot root@machine";
    const SCRIPT: &str = "#!/bin/bash\necho the builder\n";
    const IMAGE: &str =
        "https://www.googleapis.com/compute/v1/projects/debian-cloud/global/images/";

    fn pinned() -> Pinned {
        Pinned {
            script: SCRIPT.to_owned(),
            hook_key: ROOT.to_owned(),
            images: vec![format!("{IMAGE}debian-12-")],
        }
    }

    /// An instance as its maker makes it, with `changed` laid over its
    /// metadata, a value of null taking a key away.
    fn described(changed: &[(&str, Value)]) -> Value {
        let mut items = vec![
            ("block-project-ssh-keys", json!("TRUE")),
            ("enable-guest-attributes", json!("TRUE")),
            ("enable-oslogin", json!("FALSE")),
            (
                "offload-hook-key",
                json!("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIRoot"),
            ),
            (
                "offload-there-key",
                json!("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIThere"),
            ),
            ("startup-script", json!(SCRIPT)),
        ];
        for (key, value) in changed {
            items.retain(|(one, _)| one != key);
            if !value.is_null() {
                items.push((key, value.clone()));
            }
        }
        let items: Vec<Value> = items
            .iter()
            .map(|(key, value)| json!({"key": key, "value": value}))
            .collect();
        json!({
            "status": "RUNNING",
            "lastStartTimestamp": "2026-11-10T05:14:27.724-08:00",
            "metadata": {"items": items},
            "disks": [
                {"boot": false, "source": "https://x/zones/z/disks/scratch"},
                {"boot": true, "source": "https://x/zones/z/disks/offload-1"}
            ],
        })
    }

    #[test]
    fn an_instance_as_its_maker_makes_it_passes() {
        assert_eq!(
            vetted(&described(&[]), &pinned()),
            Ok("2026-11-10T05:14:27.724-08:00".to_owned()),
            "the instance as made was refused"
        );
        assert_eq!(
            vetted(&described(&[("offload-max-jobs", json!("4"))]), &pinned()),
            Ok("2026-11-10T05:14:27.724-08:00".to_owned()),
            "a VM builder's job count was refused"
        );
    }

    #[test]
    fn an_instance_changed_after_it_was_made_is_refused() {
        let changes: [(&str, (&str, Value)); 7] = [
            ("ssh keys", ("ssh-keys", json!("me:ssh-ed25519 AAAA me"))),
            ("project keys", ("block-project-ssh-keys", json!("FALSE"))),
            (
                "project keys unblocked",
                ("block-project-ssh-keys", Value::Null),
            ),
            ("OS Login", ("enable-oslogin", json!("TRUE"))),
            (
                "another hook key",
                ("offload-hook-key", json!("ssh-ed25519 AAAAC3Nza mine")),
            ),
            (
                "another script",
                ("startup-script", json!("#!/bin/bash\nmine\n")),
            ),
            ("no script", ("startup-script", Value::Null)),
        ];
        for (what, change) in changes {
            assert!(
                vetted(&described(&[change]), &pinned()).is_err(),
                "an instance with {what} passed"
            );
        }
    }

    #[test]
    fn an_instance_not_running_or_never_started_is_refused() {
        let mut stopped = described(&[]);
        stopped["status"] = json!("TERMINATED");
        assert!(
            vetted(&stopped, &pinned()).is_err(),
            "a stopped instance passed"
        );
        let mut unsaid = described(&[]);
        unsaid["lastStartTimestamp"] = Value::Null;
        assert!(
            vetted(&unsaid, &pinned()).is_err(),
            "an instance with no start passed"
        );
    }

    #[test]
    fn an_instance_is_marked_by_both_its_fingerprints() {
        let marked = json!({"fingerprint": "Kk2h", "metadata": {"fingerprint": "MQpH"}});
        assert_eq!(marks(&marked).as_deref(), Some("Kk2h MQpH"));
        assert_eq!(
            marks(&json!({"fingerprint": "Kk2h"})),
            None,
            "no metadata's"
        );
        assert_eq!(marks(&described(&[])), None, "none at all");
    }

    #[test]
    fn a_project_may_set_only_what_each_builder_overrides() {
        let project = |keys: &[&str]| {
            let items: Vec<Value> = keys
                .iter()
                .map(|key| json!({"key": key, "value": "x"}))
                .collect();
            json!({"commonInstanceMetadata": {"items": items}})
        };
        assert_eq!(
            project_holds(&project(&["enable-oslogin", "ssh-keys"])),
            Ok(())
        );
        assert_eq!(
            project_holds(&json!({})),
            Ok(()),
            "a project setting nothing"
        );
        for other in ["startup-script-url", "enable-osconfig"] {
            let refused = project_holds(&project(&["ssh-keys", other]));
            assert!(
                refused.is_err_and(|why| why.contains(other)),
                "a project setting {other} was taken"
            );
        }
    }

    #[test]
    fn only_an_instance_its_making_alone_has_touched_passes() {
        let entry = |method: &str, resource: &str| {
            json!({
                "timestamp": "2026-11-11T00:11:26Z",
                "protoPayload": {"methodName": method, "resourceName": resource},
            })
        };
        let made = entry(
            "v1.compute.instances.insert",
            "projects/p/zones/z/instances/o-1",
        );
        assert_eq!(made_alone(&json!([made, made])), Ok(()));
        let changed = json!([
            entry(
                "v1.compute.instances.setMetadata",
                "projects/p/zones/z/instances/o-1"
            ),
            made,
        ]);
        let refused = made_alone(&changed);
        assert!(
            refused.is_err_and(|why| why.contains("setMetadata at 2026-11-11T00:11:26Z")),
            "an instance changed once made passed"
        );
        let disk = json!([
            made,
            entry("v1.compute.disks.setLabels", "projects/p/zones/z/disks/o-1")
        ]);
        assert!(made_alone(&disk).is_err(), "a boot disk changed passed");
        let early = made_alone(&json!([]));
        assert!(
            early.is_err_and(|why| why.contains("isn't in the audit log")),
            "none yet"
        );
        assert!(made_alone(&json!({})).is_err(), "an unreadable log passed");
    }

    #[test]
    fn the_boot_disk_is_found_and_held_to_its_image() {
        assert_eq!(
            boot_disk(&described(&[])),
            Some("offload-1".to_owned()),
            "the boot disk was missed"
        );
        let disk = |image: &str| json!({"sourceImage": image});
        assert_eq!(
            imaged(
                &disk(&format!("{IMAGE}debian-12-bookworm-v20261104")),
                &pinned()
            ),
            Ok(()),
            "the allowed image was refused"
        );
        for image in [
            "https://www.googleapis.com/compute/v1/projects/mine/global/images/debian-12-x",
            "",
        ] {
            assert!(
                imaged(&disk(image), &pinned()).is_err(),
                "a disk from {image:?} passed"
            );
        }
    }

    #[test]
    fn a_cloud_timestamp_is_read_in_its_own_zone() {
        assert_eq!(
            seconds("1970-01-01T00:00:00Z"),
            Some(0),
            "the epoch was misread"
        );
        assert_eq!(
            seconds("2000-03-01T00:00:00.5Z"),
            Some(951_868_800),
            "a leap year's March was misread"
        );
        assert_eq!(
            seconds("2026-11-10T19:32:10.212-08:00"),
            seconds("2026-11-11T03:32:10Z"),
            "an offset wasn't taken"
        );
        assert_eq!(seconds("yesterday"), None, "no timestamp was read as one");
    }

    #[test]
    fn only_an_instance_started_once_soon_after_its_making_passes() {
        fn made(made: &str, started: &str) -> Value {
            json!({
                "creationTimestamp": made,
                "lastStartTimestamp": started,
            })
        }
        assert_eq!(
            started_once(&made(
                "2026-11-10T19:32:04.119-08:00",
                "2026-11-10T19:32:10.212-08:00"
            )),
            Ok(()),
            "a first boot was refused"
        );
        assert_eq!(
            started_once(&made(
                "2026-11-11T08:08:34.249-08:00",
                "2026-11-11T08:10:59.391-08:00"
            )),
            Ok(()),
            "a first boot at the end of a slow insert was refused"
        );
        assert_eq!(
            started_once(&made(
                "2026-11-11T08:08:34.000-08:00",
                "2026-11-11T08:18:34.000-08:00"
            )),
            Ok(()),
            "a first boot ten minutes after its making was refused"
        );
        for (was, is) in [
            (
                "2026-11-11T08:08:34.000-08:00",
                "2026-11-11T08:18:35.000-08:00",
            ),
            (
                "2026-11-10T19:32:04.119-08:00",
                "2026-11-10T19:52:10.212-08:00",
            ),
            (
                "2026-11-10T19:32:04.119-08:00",
                "2026-11-10T19:31:10.212-08:00",
            ),
            ("2026-11-10T19:32:04.119-08:00", ""),
        ] {
            assert!(
                started_once(&made(was, is)).is_err(),
                "made at {was}, started at {is:?} passed"
            );
        }
    }
}
