//! The clock beside where the laptop is: the time zone it keeps now, read
//! from the link `/etc/localtime` is, as timedated reads it, and its last
//! move to follow the laptop, as clock-follows writes it down.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

/// The zone the link names, "America/Toronto", from whatever folder of
/// zones it points into; none where it isn't a link, or names no zone.
pub fn zone(link: &Path) -> Option<String> {
    let target = fs::read_link(link).ok()?;
    let target = target.to_str()?;
    let (_, zone) = target.rsplit_once("zoneinfo/")?;
    (!zone.is_empty()).then(|| zone.to_owned())
}

/// The clock's last move, as clock-follows keeps it (`moved.json`): the
/// zone it moved from and to, the place it moved for and when, in seconds
/// since the epoch; none while it hasn't moved, or was put back.
pub fn moved(file: &Path) -> Option<Value> {
    let said: Value = serde_json::from_str(&fs::read_to_string(file).ok()?).ok()?;
    let from = said.get("from")?.as_str()?;
    let to = said.get("to")?.as_str()?;
    Some(json!({
        "from": from,
        "to": to,
        "place": said.get("place").and_then(Value::as_str),
        "at_s": said.get("when").and_then(Value::as_u64),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> std::path::PathBuf {
        let folder =
            std::env::temp_dir().join(format!("whereabouts-clock-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn the_zone_is_what_the_link_names() {
        let folder = folder("zone");
        let link = folder.join("localtime");
        std::os::unix::fs::symlink("/etc/zoneinfo/America/Halifax", &link).unwrap();
        assert_eq!(zone(&link).as_deref(), Some("America/Halifax"));
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink("../usr/share/zoneinfo/UTC", &link).unwrap();
        assert_eq!(zone(&link).as_deref(), Some("UTC"));
        fs::remove_file(&link).unwrap();
        fs::write(&link, "TZif").unwrap();
        assert_eq!(zone(&link), None, "a copy says no zone");
        assert_eq!(zone(&folder.join("nothing")), None);
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_move_is_said_in_seconds_and_none_is_none() {
        let folder = folder("moved");
        let file = folder.join("moved.json");
        assert_eq!(moved(&file), None);
        fs::write(
            &file,
            r#"{"from":"America/Toronto","to":"America/Edmonton","place":"Kelpwater, Alberta","when":1790000000}"#,
        )
        .unwrap();
        assert_eq!(
            moved(&file),
            Some(json!({
                "from": "America/Toronto",
                "to": "America/Edmonton",
                "place": "Kelpwater, Alberta",
                "at_s": 1_790_000_000_u64,
            }))
        );
        fs::write(&file, r#"{"from":"America/Toronto"}"#).unwrap();
        assert_eq!(moved(&file), None, "a move needs both ends");
        fs::remove_dir_all(&folder).unwrap();
    }
}
