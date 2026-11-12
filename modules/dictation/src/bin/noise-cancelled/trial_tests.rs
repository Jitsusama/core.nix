//! The trial's tests: what it records, refuses and offers, asked as the
//! shell asks it, with stand-ins for PipeWire's tools.

use std::sync::mpsc;

use super::*;
use crate::spot::Where;

/// A machine knowing nothing of where it is, so every spot read is
/// `network=none`.
fn nowhere() -> Where {
    Where::at(&std::env::temp_dir().join(format!("trial-nowhere-{}", std::process::id())))
}

fn trial(refusals: Vec<Refusal>) -> Arc<Shared<Trial>> {
    keeping(refusals, None)
}

/// A trial keeping its pick in `file`.
fn keeping(refusals: Vec<Refusal>, file: Option<PathBuf>) -> Arc<Shared<Trial>> {
    let tools = Tools {
        dump: "false".to_owned(),
        record: "false".to_owned(),
        play: "true".to_owned(),
        metadata: "true".to_owned(),
    };
    let running = Chosen::new(file, Cleaner::DeepFilterNet).within(nowhere());
    Trial::shared(PathBuf::new(), running, refusals, tools)
}

fn ask(trial: &Arc<Shared<Trial>>, line: &str) -> Value {
    let (told, _) = mpsc::channel();
    trial.lock().answer(line, 0, &told)
}

/// A trial with its clips recorded and cleaned, as the recording
/// thread leaves it.
fn ready(trial: &Arc<Shared<Trial>>) {
    let mut held = trial.lock();
    held.stage = Stage::Ready;
    held.microphone = Some(Microphone {
        name: "mv7".to_owned(),
        description: "Shure MV7".to_owned(),
    });
    held.clips = [Way::ALL[2], Way::ALL[0], Way::ALL[3], Way::ALL[1]]
        .into_iter()
        .map(|way| Clip {
            way,
            samples: Arc::new(vec![0.0; 10]),
        })
        .collect();
    held.ways = held.clips.iter().map(|clip| clip.way).collect();
    held.costs = Way::ALL.iter().map(|way| (way.name(), 0.1)).collect();
}

#[test]
fn it_says_the_cleaner_running_and_nothing_else_while_idle() {
    let said = ask(
        &trial(Vec::new()),
        r#"{"id":1,"ask":"watch","kind":"trial"}"#,
    );
    let record = &said["records"][0];
    assert_eq!(record["key"], KEY);
    assert_eq!(record["stage"], "idle");
    assert_eq!(record["running"], "deepfilternet");
    assert_eq!(record["seconds"], SECONDS);
    assert_eq!(record["clips"], json!([]));
    assert_eq!(record["revealed"], Value::Null);
}

#[test]
fn a_refusal_that_holds_says_why_and_records_nothing() {
    let refusals = vec![
        Refusal {
            command: vec!["false".to_owned()],
            why: "a meeting is near".to_owned(),
        },
        Refusal {
            command: vec!["true".to_owned()],
            why: "a call is on".to_owned(),
        },
    ];
    let trial = trial(refusals);
    let said = ask(&trial, r#"{"id":2,"ask":"act","act":"record"}"#);
    assert_eq!(said["error"]["code"], "refused");
    assert_eq!(said["error"]["message"], "a call is on");
    let record = trial.lock().record();
    assert_eq!(record["stage"], "refused");
    assert_eq!(record["why"], "a call is on");
}

#[test]
fn refusals_are_read_as_nix_writes_them() {
    let listed = Refusal::listed(r#"[{"command": ["a-call-is-on"], "why": "a call is on"}]"#);
    assert_eq!(
        listed,
        Ok(vec![Refusal {
            command: vec!["a-call-is-on".to_owned()],
            why: "a call is on".to_owned(),
        }])
    );
    assert!(
        Refusal::listed(r#"[{"why": "no command"}]"#).is_err(),
        "a refusal with no command was taken"
    );
}

#[test]
fn it_wont_record_twice_at_once() {
    let trial = trial(Vec::new());
    trial.lock().stage = Stage::Recording;
    let said = ask(&trial, r#"{"id":3,"ask":"act","act":"record"}"#);
    assert_eq!(said["error"]["message"], "it's already recording");
}

#[test]
fn the_clips_are_offered_blind() {
    let trial = trial(Vec::new());
    ready(&trial);
    let record = trial.lock().record();
    assert_eq!(record["clips"], json!(["A", "B", "C", "D"]));
    assert_eq!(record["revealed"], Value::Null);
    assert_eq!(record["costs"], Value::Null);
    assert_eq!(record["microphone"], "Shure MV7");
}

#[test]
fn a_pick_reveals_every_clip_and_lets_them_go() {
    let trial = trial(Vec::new());
    ready(&trial);
    let said = ask(&trial, r#"{"id":4,"ask":"act","act":"pick","clip":"D"}"#);
    assert_eq!(said["taken"], true, "{said}");
    let record = trial.lock().record();
    assert_eq!(record["stage"], "picked");
    assert_eq!(
        record["picked"],
        json!({"clip": "D", "cleaner": "deepfilternet"})
    );
    assert_eq!(
        record["revealed"],
        json!({"A": "deepfilternet-gentle", "B": "raw", "C": "rnnoise", "D": "deepfilternet"})
    );
    assert_eq!(record["costs"]["rnnoise"], 0.1);
    assert_eq!(trial.lock().clips.len(), 0, "the clips outlived the pick");
}

#[test]
fn a_cleaned_pick_is_what_the_microphones_run_and_raw_changes_nothing() {
    let file = std::env::temp_dir().join(format!("trial-pick-{}", std::process::id()));
    let _ = std::fs::remove_file(&file);
    let trial = keeping(Vec::new(), Some(file.clone()));
    ready(&trial);
    ask(&trial, r#"{"id":4,"ask":"act","act":"pick","clip":"C"}"#);
    assert_eq!(trial.lock().record()["running"], "rnnoise");
    assert_eq!(
        Chosen::new(Some(file.clone()), Cleaner::DeepFilterNet)
            .cleaner(Some("mv7"), Some(&nowhere().spot())),
        Cleaner::RNNoise,
        "the service would load some other cleaner than the one picked"
    );
    ready(&trial);
    ask(&trial, r#"{"id":5,"ask":"act","act":"pick","clip":"B"}"#);
    assert_eq!(trial.lock().record()["running"], "rnnoise");
    let _ = std::fs::remove_file(&file);
}

#[test]
fn a_pick_is_the_recorded_microphones_alone() {
    let file = std::env::temp_dir().join(format!("trial-own-{}", std::process::id()));
    std::fs::write(&file, "deepfilternet-gentle laptop\n").expect("the laptop's pick is kept");
    let trial = keeping(Vec::new(), Some(file.clone()));
    ready(&trial);
    ask(&trial, r#"{"id":4,"ask":"act","act":"pick","clip":"C"}"#);
    let chosen = Chosen::new(Some(file.clone()), Cleaner::DeepFilterNet);
    let here = nowhere().spot();
    assert_eq!(chosen.cleaner(Some("mv7"), Some(&here)), Cleaner::RNNoise);
    assert_eq!(
        chosen.cleaner(Some("laptop"), Some(&here)),
        Cleaner::DeepFilterNetGentle,
        "picking for the MV7 changed the laptop's microphone"
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn a_pick_is_kept_at_the_spot_recorded_at() {
    let file = std::env::temp_dir().join(format!("trial-spot-{}", std::process::id()));
    std::fs::write(&file, "deepfilternet-gentle mv7 home=6366013\n").expect("home's pick kept");
    let trial = keeping(Vec::new(), Some(file.clone()));
    ready(&trial);
    trial.lock().spot = Some(Spot {
        key: "network=50-wifi-Lantern".to_owned(),
        name: "away, on Lantern".to_owned(),
    });
    ask(&trial, r#"{"id":4,"ask":"act","act":"pick","clip":"C"}"#);
    let record = trial.lock().record();
    assert_eq!(record["spot"], "away, on Lantern");
    assert_eq!(record["running"], "rnnoise");
    let home = Spot {
        key: "home=6366013".to_owned(),
        name: "Main Floor".to_owned(),
    };
    assert_eq!(
        Chosen::new(Some(file.clone()), Cleaner::DeepFilterNet).cleaner(Some("mv7"), Some(&home)),
        Cleaner::DeepFilterNetGentle,
        "picking away from home changed the MV7's pick at home"
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn a_spot_is_named_as_joel_says() {
    let folder = std::env::temp_dir().join(format!("trial-named-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let trial = keeping(Vec::new(), Some(folder.join("cleaner")));
    let said = ask(&trial, r#"{"id":1,"ask":"watch","kind":"trial"}"#);
    assert_eq!(said["records"][0]["spot"], "away");
    assert_eq!(said["records"][0]["spot_named"], false);
    let said = ask(
        &trial,
        r#"{"id":2,"ask":"act","act":"name","name":"the couch"}"#,
    );
    assert_eq!(said["taken"], true, "naming was refused: {said}");
    let record = trial.lock().record();
    assert_eq!(record["spot"], "the couch");
    assert_eq!(record["spot_named"], true);
    let said = ask(&trial, r#"{"id":3,"ask":"act","act":"name","name":" "}"#);
    assert_eq!(
        said["error"]["code"], "refused",
        "a blank name was taken: {said}"
    );
    assert_eq!(trial.lock().record()["spot"], "the couch");
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn it_says_the_default_microphones_own_cleaner_running() {
    let file = std::env::temp_dir().join(format!("trial-default-{}", std::process::id()));
    std::fs::write(&file, "rnnoise mv7\ndeepfilternet-gentle laptop\n").expect("picks kept");
    let trial = keeping(Vec::new(), Some(file.clone()));
    trial.lock().default_microphone = Some("mv7".to_owned());
    assert_eq!(trial.lock().record()["running"], "rnnoise");
    trial.lock().default_microphone = Some("laptop".to_owned());
    assert_eq!(
        trial.lock().record()["running"],
        "deepfilternet-gentle",
        "Sound would say the MV7's cleaner under the laptop's microphone"
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn a_clip_nobody_has_cant_be_played_or_picked() {
    let trial = trial(Vec::new());
    let said = ask(&trial, r#"{"id":5,"ask":"act","act":"play","clip":"A"}"#);
    assert_eq!(said["error"]["message"], r#"there's no clip "A" to hear"#);
    ready(&trial);
    let said = ask(&trial, r#"{"id":6,"ask":"act","act":"pick","clip":"E"}"#);
    assert_eq!(said["error"]["code"], "refused");
}

#[test]
fn playing_says_which_clip_and_a_discard_forgets_them() {
    let trial = trial(Vec::new());
    ready(&trial);
    let said = ask(&trial, r#"{"id":7,"ask":"act","act":"play","clip":"B"}"#);
    assert_eq!(said["taken"], true, "{said}");
    assert_eq!(trial.lock().record()["playing"], "B");
    ask(&trial, r#"{"id":8,"ask":"act","act":"discard"}"#);
    let record = trial.lock().record();
    assert_eq!(record["stage"], "idle");
    assert_eq!(record["playing"], Value::Null);
    assert_eq!(
        trial.lock().clips.len(),
        0,
        "the clips outlived the discard"
    );
}

#[test]
fn every_watch_hears_each_act() {
    let trial = trial(Vec::new());
    let (told, heard) = mpsc::channel();
    trial
        .lock()
        .answer(r#"{"id":9,"ask":"watch","kind":"trial"}"#, 1, &told);
    ready(&trial);
    ask(&trial, r#"{"id":10,"ask":"act","act":"discard"}"#);
    let line: Value = serde_json::from_str(&heard.recv().unwrap()).unwrap();
    assert_eq!(line["id"], 9);
    assert_eq!(line["changed"][0]["stage"], "idle");
}

#[test]
fn the_default_microphone_is_found_through_its_twin() {
    let dump = json!([
        {"type": "PipeWire:Interface:Metadata", "props": {"metadata.name": "default"},
         "metadata": [{"subject": 0, "key": "default.audio.source",
                       "value": {"name": "noise-cancelled.mv7"}}]},
        {"info": {"props": {"node.name": "noise-cancelled.mv7",
                            "node.description": "Shure MV7, Noise Cancelled",
                            "media.class": "Audio/Source"}}},
        {"info": {"props": {"node.name": "mv7", "node.description": "Shure MV7",
                            "media.class": "Audio/Source"}}},
    ]);
    let microphone = default_microphone(&dump).expect("a microphone");
    assert_eq!(microphone.name, "mv7");
    assert_eq!(microphone.description, "Shure MV7");
    assert_eq!(microphone.twin(), "noise-cancelled.mv7");
}

#[test]
fn the_clips_play_on_the_default_output() {
    let dump = json!([
        {"type": "PipeWire:Interface:Metadata", "props": {"metadata.name": "default"},
         "metadata": [{"subject": 0, "key": "default.audio.sink",
                       "value": {"name": "headset"}}]},
        {"info": {"props": {"node.name": "speakers", "node.description": "Speakers",
                            "media.class": "Audio/Sink"}}},
        {"info": {"props": {"node.name": "headset", "node.description": "Pebble V3",
                            "media.class": "Audio/Sink"}}},
    ]);
    assert_eq!(default_output(&dump).as_deref(), Some("Pebble V3"));
}

#[test]
fn the_record_says_where_the_clips_play() {
    let trial = trial(Vec::new());
    ready(&trial);
    trial.lock().output = Some("Pebble V3".to_owned());
    assert_eq!(trial.lock().record()["output"], "Pebble V3");
}

#[test]
fn every_shuffle_offers_each_way_once() {
    let mut seen = std::collections::HashSet::new();
    for seed in 0..200 {
        let mut order = shuffled(seed * 7919);
        seen.insert(order);
        order.sort_unstable();
        assert_eq!(order, [0, 1, 2, 3], "seed {seed}");
    }
    assert!(seen.len() > 12, "only {} orders came", seen.len());
}

/// Every way's marks, as the scorer gives them, each way scoring `step`
/// more overall than the one before.
fn marked(step: f64) -> BTreeMap<&'static str, Marks> {
    Way::ALL
        .iter()
        .zip([0.0, 1.0, 2.0, 3.0])
        .map(|(way, rank)| {
            (
                way.name(),
                Marks {
                    voice: 3.0,
                    background: 2.0 + rank * step,
                    overall: 2.5 + rank * step,
                },
            )
        })
        .collect()
}

/// The lines kept in `file`, each read as JSON.
fn kept(file: &Path) -> Vec<Value> {
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line kept is JSON"))
        .collect()
}

#[test]
fn the_marks_are_revealed_only_after_a_pick() {
    let trial = trial(Vec::new());
    ready(&trial);
    let generation = trial.lock().generation;
    trial.lock().scored(generation, marked(0.5));
    assert_eq!(
        trial.lock().record()["marks"],
        Value::Null,
        "the marks could sway his ear before he picks"
    );
    ask(&trial, r#"{"id":4,"ask":"act","act":"pick","clip":"D"}"#);
    let record = trial.lock().record();
    assert_eq!(
        record["marks"]["rnnoise"],
        json!({"voice": 3.0, "background": 3.5, "overall": 4.0})
    );
    assert_eq!(record["marks"]["raw"]["overall"], 2.5);
}

#[test]
fn marks_for_a_recording_let_go_are_dropped() {
    let trial = trial(Vec::new());
    ready(&trial);
    let generation = trial.lock().generation;
    ask(&trial, r#"{"id":4,"ask":"act","act":"discard"}"#);
    trial.lock().scored(generation, marked(0.5));
    ready(&trial);
    ask(&trial, r#"{"id":5,"ask":"act","act":"pick","clip":"A"}"#);
    assert_eq!(trial.lock().record()["marks"], Value::Null);
}

#[test]
fn a_pick_keeps_its_marks_beside_it_and_never_the_sound() {
    let folder = std::env::temp_dir().join(format!("trial-marks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let trial = keeping(Vec::new(), Some(folder.join("cleaner")));
    ready(&trial);
    trial.lock().output = Some("Pebble V3".to_owned());
    let generation = trial.lock().generation;
    trial.lock().scored(generation, marked(0.5));
    ask(&trial, r#"{"id":4,"ask":"act","act":"pick","clip":"B"}"#);
    let lines = kept(&folder.join("scored"));
    assert_eq!(lines.len(), 1, "one pick keeps one line: {lines:?}");
    let line = &lines[0];
    assert_eq!(line["picked"], "raw");
    assert_eq!(line["microphone"], "Shure MV7");
    assert_eq!(line["output"], "Pebble V3");
    assert_eq!(line["marks"]["deepfilternet"]["overall"], 3.0);
    assert_eq!(line["costs"]["rnnoise"], 0.1);
    assert!(line["at"].is_u64(), "when it was picked: {line}");
    let mut said: Vec<&str> = line
        .as_object()
        .expect("a line is an object")
        .keys()
        .map(String::as_str)
        .collect();
    said.sort_unstable();
    assert_eq!(
        said,
        [
            "at",
            "costs",
            "marks",
            "microphone",
            "output",
            "picked",
            "spot"
        ],
        "a line keeps only numbers and names"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn marks_that_land_after_the_pick_are_kept_then_and_once() {
    let folder = std::env::temp_dir().join(format!("trial-late-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let trial = keeping(Vec::new(), Some(folder.join("cleaner")));
    ready(&trial);
    let generation = trial.lock().generation;
    ask(&trial, r#"{"id":4,"ask":"act","act":"pick","clip":"C"}"#);
    assert!(
        kept(&folder.join("scored")).is_empty(),
        "kept before it was scored"
    );
    trial.lock().scored(generation, marked(0.5));
    trial.lock().scored(generation, marked(0.5));
    let lines = kept(&folder.join("scored"));
    assert_eq!(
        lines.len(),
        1,
        "the late marks were kept {} times",
        lines.len()
    );
    assert_eq!(lines[0]["picked"], "rnnoise");
    assert_eq!(trial.lock().record()["marks"]["raw"]["voice"], 3.0);
    let _ = std::fs::remove_dir_all(&folder);
}
