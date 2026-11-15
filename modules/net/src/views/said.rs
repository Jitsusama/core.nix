//! What each source says about a device in its own words, past what the
//! page's tables and facts say in net's: a section per source, each path
//! its facts share said once, and what a fact can't say on a line left to
//! `--json`.

use serde_json::Value;
use terminal_kit::Pen;
use terminal_kit::layout::{self, Room};

use crate::model::Device;

/// A section for each source with something left to say about `device`,
/// headed by the source's name, in the order the record keeps them.
pub fn sections(device: &Device, pen: &Pen, room: Room) -> Vec<Vec<String>> {
    let shown = shown(device);
    let mut blocks = Vec::new();
    for (source, said) in &device.detail {
        let mut fields = Vec::new();
        match said {
            Value::Object(said) => {
                for (key, value) in said {
                    if !SAID.contains(&key.as_str()) {
                        flatten(&[key.replace('_', " ")], value, &mut fields);
                    }
                }
            }
            said => flatten(&[], said, &mut fields),
        }
        fields.retain(|(_, value)| !shown.contains(value));
        if fields.is_empty() {
            continue;
        }
        let mut block = vec![pen.label(&format!("from {source}"))];
        block.extend(said_once(&fields, pen, room));
        blocks.push(block);
    }
    blocks
}

/// Every value the page already shows, so a source's section leaves it
/// out: the device's name, its id's own part, and its generic fields.
fn shown(device: &Device) -> Vec<String> {
    let mut shown = vec![
        device.name.clone(),
        device.id.rsplit(':').next().unwrap_or_default().to_owned(),
    ];
    shown.extend(
        [
            &device.vendor,
            &device.model,
            &device.hardware,
            &device.firmware,
            &device.serial,
        ]
        .into_iter()
        .flatten()
        .cloned(),
    );
    shown.extend(device.macs.iter().cloned());
    shown.extend(device.addresses.iter().cloned());
    shown
}

/// A source's own fields that the page's tables and facts already say in
/// net's words, so its section leaves them out by name; the rest go by
/// value, through `shown`. A receiver's reads are its receiver block, but
/// for its transport's own word, and its rendering service's description
/// and the devices inside its own are left to `--json`. A printer's
/// counters and settings are left there too, since its printer block
/// says what they'd bury.
const SAID: [&str; 18] = [
    "id",
    "location",
    "model",
    "firmware",
    "serial",
    "mac",
    "address",
    "up_s",
    "ports",
    "radios",
    "GetVolume",
    "GetMute",
    "RenderingControl",
    "GetMediaInfo",
    "GetPositionInfo",
    "deviceList",
    "ProductConfigDyn",
    "ProductUsageDyn",
];

/// The fields, at any depth, that describe a source's own file rather
/// than the device, as HP's pages say their version, schema and how often
/// their alerts changed. `--json` gives them; the page leaves them out.
const ABOUT_THE_FILE: [&str; 3] = ["Version", "schemaLocation", "AlertTable"];

/// A source's facts, each path said once: a fact alone under its first
/// part keeps its whole path as its label, and those come first, lined up
/// together. Facts sharing a first part follow under a line of the path
/// they all share, two cells in, said once again in the same way at
/// every depth, so a path shared deeper in is a line of its own too.
fn said_once(fields: &[(Vec<String>, String)], pen: &Pen, room: Room) -> Vec<String> {
    let mut groups: Vec<&[(Vec<String>, String)]> = Vec::new();
    let mut start = 0;
    for at in 1..=fields.len() {
        if at == fields.len() || fields[at].0.first() != fields[start].0.first() {
            groups.push(&fields[start..at]);
            start = at;
        }
    }
    let (alone, shared): (Vec<_>, Vec<_>) = groups
        .into_iter()
        .partition(|group| group.len() < 2 || group.iter().any(|(path, _)| path.len() < 2));
    let alone: Vec<(String, &str)> = alone
        .into_iter()
        .flatten()
        .map(|(path, value)| (path.join(" "), value.as_str()))
        .collect();
    let alone: Vec<(&str, Option<&str>)> = alone
        .iter()
        .map(|(label, value)| (label.as_str(), Some(*value)))
        .collect();
    let mut lines = layout::facts(&alone, "unknown", pen, room);
    for group in shared {
        let shortest = group.iter().map(|(path, _)| path.len()).min().unwrap_or(1);
        let first = &group[0].0;
        let common = (1..shortest)
            .take_while(|&at| group.iter().all(|(path, _)| path[at] == first[at]))
            .count()
            + 1;
        lines.push(pen.role("text", &first[..common].join(" ")));
        let inset = " ".repeat(INSET);
        let rest: Vec<(Vec<String>, String)> = group
            .iter()
            .map(|(path, value)| (path[common..].to_vec(), value.clone()))
            .collect();
        lines.extend(
            said_once(&rest, pen, room.less(INSET))
                .into_iter()
                .map(|line| format!("{inset}{line}")),
        );
    }
    lines
}

/// How far a fact sits in under the path it shares with those beside it.
const INSET: usize = 2;

/// The most items a list may hold and still be a fact on one line; a
/// longer one is a series, such as a history of readings, which only
/// `--json` gives whole.
const LIST_AT_MOST: usize = 4;

/// The lists of records a source's section reads field by field, since
/// they hold its own words for what the role's block says in net's: a
/// printer's statuses, as HP names them. Every other list of records is a
/// table the page would bury its facts under, so it's left to `--json`.
const READ_BY_FIELD: [&str; 1] = ["Status"];

/// A source's fields as labels and words, a nested field labelled by the
/// path to it, part by part: `backhaul`, `upstream`. A list of flat
/// records named in `READ_BY_FIELD` reads as each field's values in turn.
/// What a fact can't say on a line, any other list of records or a
/// series, what describes the source's file, and what the source left
/// empty, are left to `--json`.
fn flatten(path: &[String], value: &Value, out: &mut Vec<(Vec<String>, String)>) {
    let label = |key: &str| {
        let mut path = path.to_vec();
        path.push(key.replace('_', " "));
        path
    };
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if !ABOUT_THE_FILE.contains(&key.as_str()) {
                    flatten(&label(key), value, out);
                }
            }
        }
        Value::Array(items) => {
            let records = items.iter().any(|item| item.is_object() || item.is_array());
            if items.is_empty() || items.len() > LIST_AT_MOST {
            } else if !records {
                let words: Vec<String> = items.iter().map(scalar).collect();
                out.push((path.to_vec(), words.join(", ")));
            } else if let Some(fields) = flat_records(items).filter(|_| {
                path.last()
                    .is_some_and(|last| READ_BY_FIELD.contains(&last.as_str()))
            }) {
                for field in fields {
                    let words: Vec<String> = items
                        .iter()
                        .filter_map(|item| item.get(&field))
                        .map(scalar)
                        .collect();
                    out.push((label(&field), words.join(", ")));
                }
            }
        }
        Value::Null => {}
        Value::String(words) if words.is_empty() => {}
        _ => out.push((path.to_vec(), scalar(value))),
    }
}

/// The fields of records that hold only words and figures, in the order
/// first met, or none when a record holds more than that.
fn flat_records(items: &[Value]) -> Option<Vec<String>> {
    let mut fields: Vec<String> = Vec::new();
    for item in items {
        for (key, value) in item.as_object()? {
            if value.is_object() || value.is_array() {
                return None;
            }
            if !fields.contains(key) {
                fields.push(key.clone());
            }
        }
    }
    Some(fields)
}

fn scalar(value: &Value) -> String {
    match value {
        Value::Bool(true) => "yes".to_owned(),
        Value::Bool(false) => "no".to_owned(),
        Value::String(words) => words.clone(),
        Value::Null => "none".to_owned(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use terminal_kit::Pen;
    use terminal_kit::layout::Room;

    use crate::views::device::device;
    use crate::views::find;
    use crate::views::tests::{ROOMS, fits, network, now};

    #[test]
    fn a_source_leaves_its_records_and_series_to_json() {
        let network = network();
        let study = find(&network, "Study").expect("the study's eero");
        let text = device(&network, study, now(), &Pen::plain(), Room::Unbounded);
        let from = text.find("FROM EERO").expect("the eero's section");
        let section = &text[from..];
        for left in ["{", "download hours", "history at", "up s", "support ends"] {
            assert!(
                !section.contains(left),
                "{left:?} is in the section: {section}"
            );
        }
        assert!(
            section.contains("ipv6"),
            "a short list is a fact: {section}"
        );
        assert!(
            section.contains("\nusage\n") && section.contains("  download total bytes"),
            "its usage's totals aren't under its usage: {section}"
        );
    }

    #[test]
    fn a_path_its_facts_share_is_said_once_above_them() {
        let network = network();
        let printer = find(&network, "Printer").expect("the printer");
        for room in ROOMS {
            let text = device(&network, printer, now(), &Pen::plain(), room);
            fits(&text, room);
            let lines: Vec<&str> = text.lines().collect();
            let head = lines
                .iter()
                .position(|line| *line == "ConsumableConfigDyn ProductConsumableInfo")
                .unwrap_or_else(|| {
                    panic!("the path its facts share isn't a line of its own: {text}")
                });
            let deeper = lines[head + 1..]
                .iter()
                .position(|line| *line == "  RewardsRegistrationStatus")
                .unwrap_or_else(|| {
                    panic!("a path shared deeper in isn't a line of its own: {text}")
                })
                + head
                + 1;
            assert!(
                lines[deeper + 1..deeper + 4]
                    .iter()
                    .all(|line| line.starts_with("    ")),
                "the facts sharing a deeper path aren't two cells further in: {text}"
            );
            assert!(
                lines[deeper + 1..]
                    .iter()
                    .any(|line| line.trim_start().starts_with("PromptAutoSendData")),
                "a fact under it isn't named by the rest of its path: {text}"
            );
            if room == Room::Cells(80) {
                assert!(
                    lines.contains(&"  AlignmentMode                       semiAutomatic"),
                    "a short fact that fits beside its label went under it: {text}"
                );
            }
            assert!(
                !text.contains("ConsumableConfigDyn ProductConsumableInfo AlignmentMode"),
                "the shared path is said again on a fact: {text}"
            );
            assert!(
                lines.contains(&"ProductStatusDyn Status"),
                "HP's statuses don't share their path: {text}"
            );
            assert!(!text.contains('\u{2026}'), "a label is cut: {text}");
        }
    }

    #[test]
    fn what_describes_a_makers_file_is_left_to_json() {
        let network = network();
        let printer = find(&network, "Printer").expect("the printer");
        let text = device(&network, printer, now(), &Pen::plain(), Room::Unbounded);
        for about_the_file in [
            "schemaLocation",
            "Version",
            "Revision",
            "ModificationNumber",
        ] {
            assert!(
                !text.contains(about_the_file),
                "{about_the_file} describes HP's file, not the printer: {text}"
            );
        }
        let json = serde_json::to_string(&printer.detail).expect("its detail as JSON");
        assert!(
            json.contains("schemaLocation"),
            "--json lost what describes HP's file: {json}"
        );
    }
}
