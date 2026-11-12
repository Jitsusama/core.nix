//! The trial's tests: what it records, refuses and offers, asked as the
//! shell asks it, with stand-ins for PipeWire's tools.

use std::sync::mpsc;

use super::*;

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
    let running = Chosen::new(file, Cleaner::DeepFilterNet);
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
        Chosen::new(Some(file.clone()), Cleaner::DeepFilterNet).cleaner(),
        Cleaner::RNNoise,
        "the service would load some other cleaner than the one picked"
    );
    ready(&trial);
    ask(&trial, r#"{"id":5,"ask":"act","act":"pick","clip":"B"}"#);
    assert_eq!(trial.lock().record()["running"], "rnnoise");
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
