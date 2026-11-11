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
}
