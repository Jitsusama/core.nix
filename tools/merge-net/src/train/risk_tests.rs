//! What the train tells the dates ledger when a red fells its riders or a
//! split takes some of them, each rider by its pull request.

use super::tests::{merging, rider, root};
use super::*;

/// The lines the dates ledger was told in `file`, as JSON.
fn told_lines(file: &Path) -> Vec<Value> {
    fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn a_red_tells_the_ledger_of_each_rider_it_fells() {
    let root = root("red-told");
    board(&root, &rider("a1", 10)).unwrap();
    let file = root.join("mission/risks.jsonl");
    let risks = Risks::new(Some(file.clone()), "Jitsusama/core.nix");
    let told = Told {
        risks: &risks,
        pulls: &[merging("a1", 6900)],
    };
    let mut splitting = Splitting::default();
    let log = Path::new("/state/train/c1.log");
    red(&root, "c1", log, &["a1".to_owned()], &mut splitting, &told);
    let lines = told_lines(&file);
    assert_eq!(lines.len(), 1, "a red that felled one rider told {lines:?}");
    let line = &lines[0];
    assert_eq!(
        (
            &line["risk"],
            &line["pr"],
            &line["run"],
            &line["eta_added_min"],
            &line["repo"]
        ),
        (
            &json!("red"),
            &json!(6900),
            &json!("c1"),
            &json!(60),
            &json!("Jitsusama/core.nix")
        ),
        "the red wasn't told on the rider's pull request at its candidate: {line}"
    );
    assert_eq!(
        line["why"].as_str(),
        fell(&root, "a1").as_deref(),
        "the red's line didn't say why the rider fell, as its fell file does"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_split_tells_the_ledger_of_each_rider_in_it() {
    let root = root("split-told");
    for (head, at) in [("a1", 10), ("b2", 20), ("c3", 30)] {
        board(&root, &rider(head, at)).unwrap();
    }
    let file = root.join("mission/risks.jsonl");
    let risks = Risks::new(Some(file.clone()), "Jitsusama/core.nix");
    let told = Told {
        risks: &risks,
        pulls: &[merging("a1", 6900), merging("b2", 6901)],
    };
    let mut splitting = Splitting::default();
    let merged: Vec<String> = ["a1", "b2", "c3"].map(str::to_owned).into();
    let log = Path::new("/state/train/c1.log");
    red(&root, "c1", log, &merged, &mut splitting, &told);
    assert_eq!(riders(&root).len(), 3, "a split dropped a rider");
    let lines = told_lines(&file);
    let told: Vec<(&Value, &Value, &Value, &Value)> = lines
        .iter()
        .map(|line| {
            (
                &line["risk"],
                &line["pr"],
                &line["run"],
                &line["eta_added_min"],
            )
        })
        .collect();
    assert_eq!(
        told,
        vec![
            (&json!("split"), &json!(6900), &json!("c1"), &json!(45)),
            (&json!("split"), &json!(6901), &json!("c1"), &json!(45)),
            (&json!("split"), &Value::Null, &json!("c1"), &json!(45)),
        ],
        "the split wasn't told once for each of its riders, by pull request where known"
    );
    assert!(
        lines.iter().all(|line| line["why"]
            .as_str()
            .is_some_and(|why| why.contains("first 1 of 3 riders"))),
        "a split's line didn't say how the train splits it: {lines:?}"
    );
    fs::remove_dir_all(&root).unwrap();
}
