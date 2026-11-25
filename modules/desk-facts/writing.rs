//! Writing each kind's facts as the launcher reads them, in JSON, to a file
//! only when what it says changes.

use std::fmt::Write;
use std::path::Path;
use std::{fs, io};

use crate::Facts;

fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Devices and their facts as the launcher reads them.
pub(crate) fn json(devices: &[(String, Facts)]) -> String {
    let mut out = String::from("{");
    for (at, (key, facts)) in devices.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        let _ = write!(out, "\n  {}: [", quoted(key));
        for (each, (name, value)) in facts.iter().enumerate() {
            if each > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "\n    {{\"name\": {}, \"value\": {}}}",
                quoted(name),
                quoted(value)
            );
        }
        out.push_str("\n  ]");
    }
    out.push_str("\n}\n");
    out
}

/// Writes a file whole, through a copy renamed over it, unless it already
/// says this. The copy is made in a `.writing` folder beside it: the shell
/// watches the desk's folder, so a copy made there woke it for each step,
/// where made aside only the rename is heard.
pub(crate) fn write(path: &Path, text: &str) -> io::Result<()> {
    if fs::read_to_string(path).is_ok_and(|said| said == text) {
        return Ok(());
    }
    let (Some(folder), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "no file named"));
    };
    let aside = folder.join(".writing");
    fs::create_dir_all(&aside)?;
    let copy = aside.join(name);
    fs::write(&copy, text)?;
    fs::rename(&copy, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_lists_each_device_s_facts_in_order_and_escapes_them() {
        let devices = vec![(
            "DP-4".to_owned(),
            vec![
                ("maker".to_owned(), "HP".to_owned()),
                ("model".to_owned(), "a \"quoted\" \\ name".to_owned()),
            ],
        )];
        assert_eq!(
            json(&devices),
            "{\n  \"DP-4\": [\n    {\"name\": \"maker\", \"value\": \"HP\"},\n    {\"name\": \"model\", \"value\": \"a \\\"quoted\\\" \\\\ name\"}\n  ]\n}\n"
        );
        assert_eq!(json(&[]), "{\n}\n");
    }

    #[test]
    fn a_file_is_written_aside_and_only_when_it_changes() {
        let folder = std::env::temp_dir().join(format!("desk-aside-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("sound.json");
        write(&path, "{}\n").unwrap();
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        write(&path, "{}\n").unwrap();
        let mut names: Vec<String> = fs::read_dir(&folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        let aside = fs::read_dir(folder.join(".writing")).unwrap().count();
        let again = fs::metadata(&path).unwrap().modified().unwrap();
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(names, [".writing", "sound.json"]);
        assert_eq!(aside, 0);
        assert_eq!(modified, again);
    }
}
