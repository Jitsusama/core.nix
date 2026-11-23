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
//! hold no script the guest agent would run at boot, no ssh keys it
//! would make accounts of and no OS Login, unless the builder sets its
//! own, and no OS Config, nor any of its agent's settings, that would run
//! jobs. Keys are read with their case folded, as the agents' Go decoding
//! folds it. The cloud allows a key only `[a-zA-Z0-9_-]`, which ASCII's
//! fold covers whole; one outside it, which Go's fold could read as
//! another (`ſ` as `s`), counts as a setting too. The ssh-key and OS Login
//! checks can't fire for a builder [`vetted`] passed, which turns both
//! off, and stay as a second line.
//!
//! A builder seen under any such setting is trusted no more, whether root
//! trusted it yet or not: a boot script may have run at a boot the cloud
//! doesn't count as a start, as a reboot from inside the guest is, and
//! what an OS Config job ran as root stays on its disk once the setting
//! goes, logged under the project rather than the instance. Every builder
//! root trusts goes with it, since the project is every builder's. So VM
//! Manager or OS inventory turned on project-wide, as another product
//! might, takes every builder away: each comes back as its lender replaces
//! it, within a lease, or at once if root, or the service's own account,
//! empties the spoiled file and restarts the service. That cost is taken
//! over trusting what a job may have changed. Any other project key is let
//! through and said, since a cloud product such as GKE writes keys of its
//! own there that no guest agent reads, and refusing on one stopped every
//! builder on 22 November.
//!
//! What root sees of the project is what it reads at each look, which
//! leaves two windows the audit log could close but doesn't, since every
//! `gcloud compute ssh` in the project writes its metadata too. A setting
//! put on and taken off between two looks, at most 30 seconds apart while
//! root's own files name a builder and 5 minutes while only the lent file
//! does, goes unseen, and OS Config needs no boot to act within one. And a
//! boot script set for a builder's first boot and taken off before root
//! first looks at it goes unseen too, since the account that makes a
//! builder also says when root looks. Both are recorded with the machine's
//! security review rather than closed here.
//!
//! Long because its tests, about half of it, stay beside the private
//! checks they read, one case each.

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

/// The project's metadata a lent builder sits under unremarked, once the
/// instance's own `block-project-ssh-keys` and `enable-oslogin` turn it
/// off, or root has judged it. Its keys, as every project key below, are
/// held with their case folded, as a project's are read.
const PROJECT_METADATA: [&str; 3] = ["enable-oslogin", "ssh-keys", "sshkeys"];

/// Whether `said` is true as the guest agents read it: they are written
/// in Go and take a boolean with its `strconv.ParseBool`, so `1` and `t`
/// turn a setting on, and `tRuE` is no boolean at all.
fn go_true(said: &str) -> bool {
    matches!(said, "1" | "t" | "T" | "TRUE" | "true" | "True")
}

/// Whether `said` is false as the guest agents read it, as `go_true`.
fn go_false(said: &str) -> bool {
    matches!(said, "0" | "f" | "F" | "FALSE" | "false" | "False")
}

/// The project's scripts the guest agent runs on an instance that doesn't
/// set its own of the same name, which would hand every builder to
/// whoever wrote one.
const PROJECT_SCRIPTS: [&str; 4] = [
    "shutdown-script",
    "shutdown-script-url",
    "startup-script",
    "startup-script-url",
];

/// The project's ssh keys, under both names the guest agent reads, which
/// it makes sudoers of on an instance not blocking them, `sshKeys` folded.
const PROJECT_SSH_KEYS: [&str; 2] = ["ssh-keys", "sshkeys"];

/// What the OS Config agent's own settings start with, under both the
/// names it reads: some turn on its patch jobs and package policies, which
/// run as root, apart from [`OS_CONFIG_SWITCHES`].
const OS_CONFIG_SETTINGS: [&str; 2] = ["osconfig-", "os-config-"];

/// The OS Config agent's switches that neither of its prefixes catch:
/// `enable-osconfig` itself, its inventory's under both names, the
/// package and patch switches older agents read, and its debug log. A
/// project may set one only to a false the agent reads, since how it
/// takes anything else isn't known here.
const OS_CONFIG_SWITCHES: [&str; 6] = [
    "enable-os-config-debug",
    "enable-os-inventory",
    "enable-osconfig",
    "os-inventory-enabled",
    "os-package-enabled",
    "os-patch-enabled",
];

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
    if !value("block-project-ssh-keys").is_some_and(go_true) {
        return Err("takes the project's ssh keys".to_owned());
    }
    // Anything but a false the agent reads, `1` among them, may turn OS
    // Login on, and a builder saying nothing takes the project's, so only
    // a false passes, as its maker sets.
    if !value("enable-oslogin").is_some_and(go_false) {
        return Err("doesn't turn OS Login off, so it could make accounts".to_owned());
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

/// The metadata `items`, as the cloud lists them in JSON.
fn items(items: &Value) -> &[Value] {
    items.as_array().map_or(&[][..], Vec::as_slice)
}

/// The keys the metadata `items` set, in order.
fn keys(items: &Value) -> Vec<String> {
    self::items(items)
        .iter()
        .filter_map(|item| item["key"].as_str().map(str::to_owned))
        .collect()
}

/// Whether the metadata `items` set `key` to a value `is` holds of.
fn said(items: &Value, key: &str, is: fn(&str) -> bool) -> bool {
    self::items(items)
        .iter()
        .any(|item| item["key"] == key && item["value"].as_str().is_some_and(is))
}

/// Whether every one of the metadata `items` setting `key` sets it to a
/// value `is` holds of, so one said twice, as case folding makes of
/// `enable-osconfig` and `Enable-OsConfig`, passes only when both do.
fn only_said(items: &Value, key: &str, is: fn(&str) -> bool) -> bool {
    self::items(items)
        .iter()
        .filter(|item| item["key"] == key)
        .all(|item| item["value"].as_str().is_some_and(is))
}

/// The project `described`'s metadata items with their keys' case folded,
/// as the guest agents' Go decoding reads them into their settings.
fn folded(described: &Value) -> Value {
    items(&described["commonInstanceMetadata"]["items"])
        .iter()
        .map(|item| {
            let mut item = item.clone();
            if let Some(key) = item["key"].as_str() {
                item["key"] = Value::from(key.to_ascii_lowercase());
            }
            item
        })
        .collect()
}

/// The keys the project `described` sets that root doesn't know what they
/// do, which pass and are said, whatever else it holds, as it spells them.
pub fn project_unknown(described: &Value) -> Vec<String> {
    keys(&described["commonInstanceMetadata"]["items"])
        .into_iter()
        .filter(|key| {
            let key = key.to_ascii_lowercase();
            let key = key.as_str();
            !PROJECT_SCRIPTS.contains(&key)
                && !PROJECT_METADATA.contains(&key)
                && !OS_CONFIG_SWITCHES.contains(&key)
                && !OS_CONFIG_SETTINGS
                    .iter()
                    .any(|setting| key.starts_with(setting))
        })
        .collect()
}

/// A project setting that would act on a builder.
#[derive(Debug, PartialEq, Eq)]
pub struct Sits {
    /// What it sets and what that would do, to follow the builder's name.
    pub why: String,
}

/// Whether `key` is one the cloud's metadata allows, `[a-zA-Z0-9_-]`.
fn plain(key: &str) -> bool {
    key.bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

/// Whether the project `described` sets no boot script, ssh keys, OS Login
/// or OS Config the instance `instance` would take; what it sets otherwise.
/// A boot script counts where the instance sets none of the same name, as
/// the guest agent runs it then. OS Config is refused whatever the builder
/// sets, since no builder may carry `enable-osconfig` of its own.
pub fn project_holds(described: &Value, instance: &Value) -> Result<(), Sits> {
    let (project, builder) = (&folded(described), &instance["metadata"]["items"]);
    let sits = |key: &str, what: &str| {
        Err(Sits {
            why: format!("sits in a project setting {key}, {what}"),
        })
    };
    // Folded only in case, as a defence: [`vetted`] lets a builder carry
    // none but its maker's own keys, as they're spelt.
    let own: Vec<String> = keys(builder)
        .iter()
        .map(|key| key.to_ascii_lowercase())
        .collect();
    for key in keys(project) {
        let named = key.as_str();
        if !plain(named) {
            let what =
                "whose name the cloud allows no key, so what an agent reads it as is unknown";
            return sits(named, what);
        }
        if OS_CONFIG_SETTINGS
            .iter()
            .any(|setting| named.starts_with(setting))
            || (OS_CONFIG_SWITCHES.contains(&named) && !only_said(project, named, go_false))
        {
            let what = "which could let OS Config run jobs on every builder";
            return sits(named, what);
        }
        if PROJECT_SCRIPTS.contains(&named) && !own.contains(&key) {
            return sits(named, "which every builder would run");
        }
        if PROJECT_SSH_KEYS.contains(&named) && !said(builder, "block-project-ssh-keys", go_true) {
            return sits(named, "whose keys every builder would take");
        }
    }
    if said(project, "enable-oslogin", go_true) && !said(builder, "enable-oslogin", go_false) {
        let what = "which would let OS Login make accounts on a builder not turning it off";
        return sits("enable-oslogin", what);
    }
    Ok(())
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
    fn a_flag_reads_as_gos_parse_bool_reads_it() {
        for on in ["1", "t", "T", "TRUE", "true", "True"] {
            assert!(go_true(on) && !go_false(on), "{on} wasn't read as true");
        }
        for off in ["0", "f", "F", "FALSE", "false", "False"] {
            assert!(go_false(off) && !go_true(off), "{off} wasn't read as false");
        }
        for neither in ["tRuE", "yes", "no", "", " true", "x"] {
            assert!(
                !go_true(neither) && !go_false(neither),
                "{neither:?} was read as a boolean"
            );
        }
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
        let changes: [(&str, (&str, Value)); 11] = [
            ("ssh keys", ("ssh-keys", json!("me:ssh-ed25519 AAAA me"))),
            ("project keys", ("block-project-ssh-keys", json!("FALSE"))),
            (
                "project keys blocked as no agent reads",
                ("block-project-ssh-keys", json!("tRuE")),
            ),
            ("OS Login said 1", ("enable-oslogin", json!("1"))),
            ("OS Login said oddly", ("enable-oslogin", json!("no"))),
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
            ("OS Login unsaid", ("enable-oslogin", Value::Null)),
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

    /// Metadata items setting each of `pairs`.
    fn pairs(pairs: &[(&str, &str)]) -> Vec<Value> {
        pairs
            .iter()
            .map(|(key, value)| json!({"key": key, "value": value}))
            .collect()
    }

    /// A project setting each of `keys` to `x`.
    fn project(keys: &[&str]) -> Value {
        let set: Vec<(&str, &str)> = keys.iter().map(|key| (*key, "x")).collect();
        json!({"commonInstanceMetadata": {"items": pairs(&set)}})
    }

    /// A project setting `key` to `value` alone.
    fn setting(key: &str, value: &str) -> Value {
        json!({"commonInstanceMetadata": {"items": pairs(&[(key, value)])}})
    }

    /// A builder as made, blocking the project's ssh keys and OS Login.
    fn builder() -> Value {
        json!({"metadata": {"items": pairs(&[
            ("block-project-ssh-keys", "TRUE"),
            ("enable-oslogin", "FALSE"),
            ("startup-script", "#!/bin/bash\n"),
        ])}})
    }

    /// A builder blocking nothing of the project's.
    fn open() -> Value {
        json!({"metadata": {"items": pairs(&[("startup-script", "#!/bin/bash\n")])}})
    }

    #[test]
    fn a_project_setting_only_what_a_builder_overrides_holds() {
        let overridden = json!({"commonInstanceMetadata": {"items": pairs(&[
            ("enable-oslogin", "TRUE"),
            ("ssh-keys", "x"),
        ])}});
        assert_eq!(
            project_holds(&overridden, &builder()),
            Ok(()),
            "what every builder overrides was refused"
        );
        assert_eq!(
            project_holds(&json!({}), &builder()),
            Ok(()),
            "a project setting nothing was refused"
        );
        assert_eq!(
            project_holds(&project(&["startup-script"]), &builder()),
            Ok(()),
            "a script the builder sets its own of was refused"
        );
        for off in ["FALSE", "false", "0", "f"] {
            for switch in OS_CONFIG_SWITCHES {
                assert_eq!(
                    project_holds(&setting(switch, off), &builder()),
                    Ok(()),
                    "a project's {switch}, said {off}, was refused"
                );
            }
        }
    }

    #[test]
    fn a_project_may_set_no_boot_script_a_builder_would_run() {
        for script in PROJECT_SCRIPTS
            .iter()
            .filter(|key| **key != "startup-script")
        {
            let refused = project_holds(&project(&["ssh-keys", script]), &builder());
            assert!(
                refused.is_err_and(|sits| sits.why.contains(script)),
                "a project's {script} was let run"
            );
        }
        assert!(
            project_holds(&project(&["startup-script"]), &json!({"metadata": {}})).is_err(),
            "a project's startup script ran on a builder setting none"
        );
    }

    // OS Config is refused unless a switch is said false as the agent
    // reads it, since how it takes anything else isn't known here.
    #[test]
    fn a_project_may_set_no_os_config_but_off() {
        for setting in [
            "osconfig-enabled-prerelease-features",
            "os-config-log-level",
        ] {
            assert!(
                project_holds(&project(&[setting]), &builder())
                    .is_err_and(|sits| sits.why.contains(setting)),
                "a project's OS Config setting {setting} was let through"
            );
        }
        for said in ["TRUE", "true", "1", "t", "x", "yes", ""] {
            for switch in OS_CONFIG_SWITCHES {
                assert!(
                    project_holds(&setting(switch, said), &builder())
                        .is_err_and(|sits| sits.why.contains(switch)),
                    "a project's {switch}, said {said:?}, reached a builder"
                );
            }
        }
    }

    #[test]
    fn a_project_may_set_no_ssh_key_or_os_login_a_builder_would_take() {
        for on in ["TRUE", "true", "1", "t"] {
            assert!(
                project_holds(&setting("enable-oslogin", on), &open())
                    .is_err_and(|sits| sits.why.contains("enable-oslogin")),
                "a project's OS Login, said {on}, reached a builder not turning it off"
            );
            assert_eq!(
                project_holds(&setting("enable-oslogin", on), &builder()),
                Ok(()),
                "a project's OS Login, said {on}, was refused though the builder turns it off"
            );
        }
        let unblocking = json!({"metadata": {"items": pairs(&[
            ("block-project-ssh-keys", "FALSE"),
            ("startup-script", "#!/bin/bash\n"),
        ])}});
        let wrongly = json!({"metadata": {"items": pairs(&[("block-project-ssh-keys", "tRuE")])}});
        for not_blocking in [&unblocking, &wrongly] {
            assert!(
                project_holds(&project(&["ssh-keys"]), not_blocking)
                    .is_err_and(|sits| sits.why.contains("ssh-keys")),
                "a project's ssh keys reached a builder whose block isn't one the agent reads"
            );
        }
        for keys in PROJECT_SSH_KEYS {
            assert!(
                project_holds(&project(&[keys]), &open())
                    .is_err_and(|sits| sits.why.contains(keys)),
                "a project's {keys} reached a builder not blocking them"
            );
            assert_eq!(
                project_holds(&project(&[keys]), &builder()),
                Ok(()),
                "a project's {keys} was refused though the builder blocks them"
            );
        }
    }

    // The agents decode metadata in Go, which matches a key to a setting
    // with its case folded, so a project's ENABLE-OSCONFIG turns OS Config
    // on as enable-osconfig does, and wins when said after it.
    #[test]
    fn a_project_key_is_judged_with_its_case_folded() {
        for key in ["ENABLE-OSCONFIG", "Osconfig-Endpoint", "Startup-Script-Url"] {
            assert!(
                project_holds(&setting(key, "TRUE"), &builder()).is_err(),
                "a project's {key} was let through"
            );
            assert!(
                project_unknown(&project(&[key])).is_empty(),
                "a project's {key} was named as a key root doesn't know"
            );
        }
        for keys in ["sshKeys", "SSH-Keys"] {
            assert!(
                project_holds(&project(&[keys]), &open()).is_err(),
                "a project's {keys} reached a builder not blocking them"
            );
            assert!(
                project_unknown(&project(&[keys])).is_empty(),
                "a project's {keys} was named as a key root doesn't know"
            );
        }
        let both = json!({"commonInstanceMetadata": {"items": pairs(&[
            ("enable-osconfig", "FALSE"),
            ("Enable-OsConfig", "TRUE"),
        ])}});
        assert!(
            project_holds(&both, &builder()).is_err(),
            "a project saying OS Config is off and on, in two cases, was let through"
        );
        assert!(
            project_holds(&setting("Enable-OSLogin", "TRUE"), &open())
                .is_err_and(|sits| sits.why.contains("enable-oslogin")),
            "a project's Enable-OSLogin reached a builder not turning OS Login off"
        );
        assert_eq!(
            project_holds(&setting("Enable-OsConfig", "FALSE"), &builder()),
            Ok(()),
            "a project's Enable-OsConfig, said false, was refused"
        );
        for key in ["\u{17f}sh-keys", "ssh-\u{212a}eys", "startup script"] {
            assert!(
                project_holds(&project(&[key]), &builder())
                    .is_err_and(|sits| sits.why.contains("allows no key")),
                "a project's {key}, which the cloud allows no key, was let through"
            );
        }
        assert_eq!(
            project_unknown(&project(&["GKE-Range"])),
            ["GKE-Range"],
            "an unknown key wasn't named as the project spells it"
        );
    }

    #[test]
    fn only_the_project_keys_root_doesnt_know_are_named() {
        let gke = "gke-a-cluster-0e6a923f-secondary-ranges";
        let mut keys = vec!["ssh-keys", gke, "startup-script-url", "os-config-log-level"];
        keys.extend(OS_CONFIG_SWITCHES);
        assert_eq!(
            project_unknown(&project(&keys)),
            [gke],
            "a key root judges was named, or one it doesn't wasn't"
        );
        assert!(
            project_unknown(&json!({})).is_empty(),
            "a project setting nothing named a key"
        );
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
