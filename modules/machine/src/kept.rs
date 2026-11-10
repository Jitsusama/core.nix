//! What Joel keeps the machine to, which the shell watches while the
//! launcher's shut: Keep Awake's two locks (`kept:idle`, `kept:lid`),
//! each held while its service leaves its file in the runtime folder
//! (modules/quickshell/home.nix's `holding`), and the power profile
//! (`profile:main`): what power-profiles-daemon is on, offers and is held
//! to, with the profile the machine chose for itself, which follow.sh
//! writes to `/run/power-profile/wanted` (modules/power-profiles-daemon).
//! Read and told here as they change, they reach the shell over the
//! socket it already holds, so it watches no file and asks no bus of its
//! own.

use std::path::Path;

use serde_json::{Value, json};

use crate::Root;

/// Each lock by its name, with the file its service leaves.
const LOCKS: [(&str, &str); 2] = [("idle", "kept-awake"), ("lid", "kept-awake-lid")];
/// The folder follow.sh writes the machine's own choice in.
pub const CHOSEN: &str = "/run/power-profile";
/// Its file there.
const WANTED: &str = "wanted";

/// Keep Awake's locks, each held while its file is in `runtime`; with
/// no runtime folder known, neither can be said.
pub fn locks(root: &Root, runtime: Option<&str>) -> Vec<Value> {
    LOCKS
        .iter()
        .map(|(lock, file)| {
            let held = runtime.map(|folder| root.path(&format!("{folder}/{file}")).exists());
            json!({"key": format!("kept:{lock}"), "kind": "kept", "lock": lock, "held": held})
        })
        .collect()
}

/// Whether a file of this name, in the runtime folder, is a lock's.
pub fn names_a_lock(name: &str) -> bool {
    LOCKS.iter().any(|(_, file)| *file == name)
}

/// Whether a file of this name, in [`CHOSEN`], is the machine's choice.
pub fn names_the_choice(name: &str) -> bool {
    name == WANTED
}

/// The power profile, as power-profiles-daemon last said it (`said`,
/// null while it hasn't answered) and as the machine chose it, null
/// where nothing chooses, as where nothing follows the charger.
pub fn profile(root: &Root, said: &Value) -> Value {
    let field = |name: &str| said.get(name).cloned().unwrap_or(Value::Null);
    let wanted = root.said(&Path::new(CHOSEN).join(WANTED).to_string_lossy());
    json!({
        "key": "profile:main",
        "kind": "profile",
        "active": field("active"),
        "profiles": field("profiles"),
        "holds": field("holds"),
        "degraded": field("degraded"),
        "wanted": wanted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lock_is_held_while_its_file_is_there() {
        let root = std::env::temp_dir().join(format!("machine-kept-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("run/user/1000")).unwrap();
        std::fs::write(root.join("run/user/1000/kept-awake"), "").unwrap();
        let said = locks(&Root::new(&root), Some("/run/user/1000"));
        let unknown = locks(&Root::new(&root), None);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(said[0], json!({"key": "kept:idle", "kind": "kept", "lock": "idle", "held": true}));
        assert_eq!(said[1]["key"], "kept:lid");
        assert_eq!(said[1]["held"], json!(false));
        assert_eq!(unknown[0]["held"], Value::Null, "no runtime folder, nothing said");
        assert!(
            names_a_lock("kept-awake-lid") && !names_a_lock("backlight-now"),
            "only a lock's own file names a lock"
        );
    }

    #[test]
    fn the_profile_says_the_daemons_and_the_machines_choice() {
        let root = std::env::temp_dir().join(format!("machine-profile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let unheard = profile(&Root::new(&root), &Value::Null);
        std::fs::create_dir_all(root.join("run/power-profile")).unwrap();
        std::fs::write(root.join("run/power-profile/wanted"), "performance\n").unwrap();
        let said = profile(
            &Root::new(&root),
            &json!({"active": "balanced", "profiles": ["balanced", "performance"]}),
        );
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(unheard["active"], Value::Null);
        assert_eq!(unheard["wanted"], Value::Null, "nothing chooses");
        assert_eq!(said["active"], "balanced");
        assert_eq!(said["profiles"], json!(["balanced", "performance"]));
        assert_eq!(said["wanted"], "performance");
        assert_eq!(said["holds"], Value::Null);
    }
}
