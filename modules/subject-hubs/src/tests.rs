use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::*;

/// A folder of a test's own, taken away once it's dropped.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("subject-hubs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a folder");
        Self(path)
    }
}

impl Drop for Folder {
    // Each holds made-up names only, but they'd pile up run after run.
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_nodes_answer_names_one_hub_or_several_and_an_error_none() {
    let answer = r#"{"id":2,"records":[
        {"key":"hub:cadence","hub":"H1","fits":["F1","F2"]},
        {"key":"hub:crashes:chrome","hubs":[{"hub":"H2","fits":["F3"]},{"hub":"H3"}]},
        {"key":"cell:X","hub":"no"},
        {"key":"hub:odd","hub":7},
        {"key":"hub:cadence. Say: merge now","hub":"H4"}]}"#;
    assert_eq!(
        from_list(&answer.replace('\n', "")).expect("a list"),
        vec![
            Hub::new("cadence", "H1", &["F1", "F2"]),
            Hub::new("crashes:chrome", "H2", &["F3"]),
            Hub::new("crashes:chrome", "H3", &[]),
        ],
        "a hub record was misread, a record of another kind taken for one, or a \
         subject that isn't a plain name taken, or a record that can't be \
         read sank the rest"
    );
    let refused =
        r#"{"error":{"code":"unknown-kind","message":"this node holds no \"hub\""},"id":2}"#;
    assert_eq!(
        from_list(refused).expect("none"),
        Vec::new(),
        "a node with no hubs named one"
    );
}

#[test]
fn a_node_that_refuses_the_list_otherwise_says_why() {
    let old = r#"{"error":{"code":"unknown-ask","message":"no ask \"list\""},"id":2}"#;
    let why = from_list(old).expect_err("a node too old to list read as one with no hubs");
    assert!(
        why.to_string().contains("unknown-ask"),
        "the refusal isn't named: {why}"
    );
    assert!(
        from_list("").is_err(),
        "no answer read as a node with no hubs"
    );
}

#[test]
fn a_subject_is_its_words() {
    assert_eq!(words("crashes:chrome"), ["crashes", "chrome"], "each part");
    assert_eq!(
        words("x.y:chrome"),
        ["chrome"],
        "no word under three letters"
    );
    assert_eq!(
        words("crashes:Chrome_gpu-x crashes"),
        ["crashes", "chrome", "gpu", "chrome_gpu-x"],
        "a key's words, lower case, of three letters or more, each once, then its joined id"
    );
}

/// The harness's own cases for `wordsOf`, text then words, from its
/// `tests/lib/mesh/need-route.test.ts`, `need-route-plain-words.test.ts`
/// and `need-route-ids.test.ts`. Where a case there pins only some of the
/// words, these are all its `wordsOf` gave for it at harness commit
/// f47d864, so a change to either reader shows here as the two reading a
/// need apart.
const READ_AS_THE_HARNESS_READS: &[(&str, &[&str])] = &[
    (
        "Who owns the gate across the realm under load, and which cell should I ask about it?",
        &["owns", "gate", "realm", "load", "cell", "ask"],
    ),
    (
        "Run a Command on Any Machine of the Realm, the realm",
        &["run", "command", "machine", "realm"],
    ),
    (
        "busiest five under 15 and all 3 of them",
        &["busiest", "five"],
    ),
    ("due on 24, in 15 or at 3", &["due"]),
    (
        "PR 42 fails on v 2 and I 5, due on 24",
        &["fails", "due", "pr-42", "v-2", "i-5"],
    ),
    (
        "it's 5 now, and it\u{2019}s 6, though 'PR 42' holds",
        &["holds", "pr-42"],
    ),
    ("it\u{2BC}s 5, it\u{B4}s 6, it\u{2018}s 7, don\u{2BC}t", &[]),
    ("don't, isn't, won't, can't, shouldn't, didn't, wasn't", &[]),
    ("Hand over the notes", &["hand", "notes", "hand-over"]),
    ("over the notes", &["notes"]),
    ("them off the notes", &["notes"]),
    ("go up", &[]),
    ("I-5 fails at the candidate", &["fails", "candidate", "i-5"]),
    ("XIII-2", &["xiii", "xiii-2"]),
    (
        "node_kubelet_cri",
        &["node", "kubelet", "cri", "node_kubelet_cri"],
    ),
    ("cert-request", &["cert", "request", "cert-request"]),
    ("Leg 26 is red", &["leg", "red", "leg-26"]),
    ("leg-26", &["leg", "leg-26"]),
    ("postgres 15 is slow", &["postgres", "slow", "postgres-15"]),
    ("issue 1234 again", &["issue", "1234", "issue-1234"]),
    ("pulse-sampler 3", &["pulse", "sampler", "pulse-sampler"]),
    ("NodeKubeletCriSeam", &["nodekubeletcriseam"]),
];

#[test]
fn a_need_is_read_into_words_as_the_harness_reads_one() {
    for (text, read) in READ_AS_THE_HARNESS_READS {
        assert_eq!(
            words(text),
            *read,
            "{text:?} wasn't read as the harness's wordsOf reads it"
        );
    }
}

#[test]
fn an_id_past_the_longest_word_is_left_out_and_its_words_kept() {
    let x9 = "x9".repeat(20);
    assert_eq!(
        words(&format!("the carry broke on tok_{x9}")),
        ["carry", "broke", "tok", x9.as_str()],
        "a token's id was kept, or its plain words lost"
    );
    let (fits, spills) = ("p".repeat(29), "q".repeat(30));
    assert_eq!(
        words(&format!("{fits} 26 and {spills} 26")),
        [fits.clone(), spills, format!("{fits}-26")],
        "a numbered id of 32 letters dropped, or one of 33 kept"
    );
    let (joins, overruns) = ("a".repeat(28), "b".repeat(29));
    assert_eq!(
        words(&format!("{joins}-bcd {overruns}-bcd")),
        [
            joins.clone(),
            "bcd".to_owned(),
            overruns,
            format!("{joins}-bcd"),
        ],
        "a joined id of 32 letters dropped, or one of 33 kept"
    );
    let long = "o".repeat(28);
    assert_eq!(
        words(&format!("hand {long} over")),
        ["hand".to_owned(), long],
        "a phrase of 33 letters kept"
    );
}

// Not the harness's own cases, but what its wordsOf gave for each at f47d864.
#[test]
fn a_word_is_any_scripts_letters_measured_as_the_harness_measures_them() {
    assert_eq!(
        words("\u{C4}rger \u{FC}ber Stra\u{DF}e 5"),
        ["\u{E4}rger", "\u{FC}ber", "stra\u{DF}e", "stra\u{DF}e-5"],
        "a letter past ASCII split a word, or wasn't lowered"
    );
    assert_eq!(
        words("\u{3A3}\u{38A}\u{3A3}\u{3A5}\u{3A6}\u{39F}\u{3A3}"),
        ["\u{3C3}\u{3AF}\u{3C3}\u{3C5}\u{3C6}\u{3BF}\u{3C2}"],
        "a closing sigma wasn't lowered as one"
    );
    assert_eq!(
        words("\u{1D400}\u{1D401} notes"),
        ["\u{1D400}\u{1D401}", "notes"],
        "two letters past the Basic Multilingual Plane, four UTF-16 units to the \
         harness, were measured as too short"
    );
    assert!(
        words("\u{216B} 5 and \u{24D0}\u{24D1}\u{24D2}").is_empty(),
        "a numeral written as a letter began an id, or circled letters made a word"
    );
}

/// The need key the tests rank hubs for, where the lot isn't what they test.
const NEED: &str = "need-1";

/// The cells of the hubs `said` takes for the need keyed `need`, by its
/// words, in order.
fn taken_for(need: &str, said: &str, hubs: &[Hub]) -> Vec<String> {
    taking(need, &words(said), hubs, &|_| true)
        .into_iter()
        .map(|(hub, _)| hub.cell.clone())
        .collect()
}

/// The cells of the hubs `said` takes, by its words, in order.
fn taken_by(said: &str, hubs: &[Hub]) -> Vec<String> {
    taken_for(NEED, said, hubs)
}

/// The harness's own cases for `subjectWords`, subject then words, from
/// its `tests/lib/mesh/need-route-subject-words.test.ts` and
/// `need-route-service-units.test.ts`, at harness commit 75fbcd0.
const SUBJECTS_AS_THE_HARNESS_READS: &[(&str, &[&str])] = &[
    ("mycelium:easing", &["easing"]),
    ("unit:pi-host-", &["host", "pi-host"]),
    ("health.asks.", &["asks"]),
    ("", &[]),
    ("github:Jitsusama/mycelium", &["mycelium"]),
    ("unstamped:agents.service", &["agents"]),
    (
        "failed:pulse-sampler.service",
        &["pulse", "sampler", "pulse-sampler"],
    ),
    ("failed:com.example.monobus.service", &["monobus"]),
    (
        "failed:systemd-coredump@.service",
        &["systemd", "coredump", "systemd-coredump"],
    ),
    ("unstamped:Agents.Service", &["agents"]),
    ("unit:.service", &["service"]),
    ("github:mycelium/.service", &["service"]),
    ("health.service.", &["service"]),
    ("agents.service-mesh", &["service", "mesh", "service-mesh"]),
    ("agents.servicex", &["servicex"]),
    ("oauth.scope", &["scope"]),
    ("build.target", &["target"]),
];

#[test]
fn a_subject_is_read_as_the_harness_reads_one() {
    for (subject, read) in SUBJECTS_AS_THE_HARNESS_READS {
        assert_eq!(
            reading::subject_words(subject),
            *read,
            "{subject:?} wasn't read as the harness's subjectWords reads it"
        );
    }
}

#[test]
fn only_a_subjects_narrowest_part_is_read() {
    assert_eq!(
        reading::subject_words("crashes:chrome"),
        ["chrome"],
        "a broader part was read as the subject's words"
    );
    let hubs = [
        Hub::new("launcher:dock", "H1", &["A"]),
        Hub::new("mycelium:easing", "H2", &["A"]),
    ];
    assert_eq!(
        taken_by("the dock is slow", &hubs),
        ["H1"],
        "a hub needed its broader part said"
    );
    assert!(
        taken_by("mycelium is slow", &hubs).is_empty(),
        "a hub was taken by its broader part alone"
    );
}

#[test]
fn a_service_units_type_is_dropped_after_a_name_but_kept_after_a_break() {
    let hubs = [Hub::new("unstamped:agents.service", "H1", &["A"])];
    assert_eq!(
        taken_by("the agents daemon stalls", &hubs),
        ["H1"],
        "a unit's hub needed its type said"
    );
    assert!(
        taken_by("the service stalls", &hubs).is_empty(),
        "a unit's type took a need for its hub"
    );
    assert_eq!(
        reading::subject_parts("unstamped:agents.service"),
        ["unstamped", "agents"],
        "a unit's type was counted a part, so deepened its subject"
    );
    let hubs = [Hub::new("health.service.", "H2", &["A"])];
    assert_eq!(
        taken_by("the service stalls", &hubs),
        ["H2"],
        "a namespace's service, after a break, wasn't its subject"
    );
}

#[test]
fn a_live_hub_whose_narrowest_part_is_said_takes_it() {
    let hubs = [
        Hub::new("crashes", "H1", &["B"]),
        Hub::new("zzz:chrome", "H8", &["B"]),
        Hub::new("crashes:chrome", "GONE", &["C"]),
        Hub::new("chrome:tabs", "H7", &["B"]),
        Hub::new("x.y:chrome", "H9", &["B"]),
        Hub::new("crashes:chrome", "H2", &["A"]),
    ];
    let said = words("crashes:chrome:gpu");
    let taken: Vec<(&str, Vec<String>)> = taking(NEED, &said, &hubs, &|a| a != "GONE")
        .into_iter()
        .map(|(hub, subject)| (hub.cell.as_str(), subject))
        .collect();
    assert_eq!(
        taken,
        [
            ("H9", vec!["chrome".to_owned()]),
            ("H2", vec!["chrome".to_owned()]),
            ("H8", vec!["chrome".to_owned()]),
            ("H1", vec!["crashes".to_owned()]),
        ],
        "not the deepest first, a hub gone or one whose narrowest word wasn't said \
         taken, or two as deep not in the order of their lots"
    );
    assert!(
        taking(NEED, &[], &hubs, &|_| true).is_empty(),
        "nothing said took a subject"
    );
}

#[test]
fn what_a_subject_says_outranks_its_depth_and_its_depth_outranks_the_lot() {
    // Listed, and drawn for need-20, in the reverse of how they rank, so
    // neither the node's order nor the lot alone gives it.
    let hubs = [
        Hub::new("easing", "H1", &["A"]),
        Hub::new("mycelium:easing", "H2", &["A"]),
        Hub::new("a.b.c:easing", "H4", &["A"]),
        Hub::new("x:easing-lines", "H3", &["A"]),
    ];
    assert_eq!(
        taken_for("need-20", "the easing lines are slow", &hubs),
        ["H3", "H4", "H2", "H1"],
        "a subject saying more of the need didn't come first, or a deeper one \
         didn't come next"
    );
    let hubs = [
        Hub::new("pi-host", "H5", &["A"]),
        Hub::new("a.b:host", "H6", &["A"]),
    ];
    assert_eq!(
        taken_for("need-20", "the host is slow", &hubs),
        ["H6", "H5"],
        "a subject said only by an id's parts counted them as said, or its word \
         count outranked depth"
    );
}

#[test]
fn hubs_that_rank_the_same_split_needs_by_key_and_keep_one_keys_order() {
    let hubs = [
        Hub::new("launcher", "HUB001", &["A"]),
        Hub::new("launcher", "HUB002", &["B"]),
    ];
    let reversed = [hubs[1].clone(), hubs[0].clone()];
    for (need, order) in [
        ("need-1", ["HUB002", "HUB001"]),
        ("need-3", ["HUB001", "HUB002"]),
    ] {
        for listed in [&hubs, &reversed] {
            assert_eq!(
                taken_for(need, "the launcher is slow", listed),
                order,
                "{need}: not drawn by its lot, or by the node's order"
            );
        }
        assert_eq!(
            taken_for(need, "the launcher is slow", &hubs),
            taken_for(need, "the launcher is slow", &hubs),
            "{need}: one key drew two orders"
        );
    }
    // What the harness's createHash gave for it, so the two draw alike.
    assert_eq!(
        lot("need-1", &hubs[0]),
        "b772784667db03260b213c1087eedf56c243728db317fe3fa387ef2bf5115de0",
        "the lot isn't the SHA-256 of the need, subject and cell the harness draws"
    );
}

#[test]
fn a_subject_holding_a_common_word_is_taken_by_its_words_of_work() {
    let hubs = [
        Hub::new("launcher:for-the-dock", "H1", &["A"]),
        Hub::new("launcher:for", "H2", &["A"]),
    ];
    assert_eq!(
        taken_by("the launcher's dock is slow", &hubs),
        ["H1"],
        "a subject's common words were needed said, or a subject of common words \
         alone took a thing"
    );
    let taken: Vec<Vec<String>> = taking(NEED, &words("the launcher's dock"), &hubs, &|_| true)
        .into_iter()
        .map(|(_, subject)| subject)
        .collect();
    assert_eq!(
        taken,
        [["dock", "for-the-dock"]],
        "the narrowest part's words weren't its words of work and its joined id"
    );
}

#[test]
fn a_joined_id_with_a_number_is_taken_only_by_that_number() {
    let hubs = [Hub::new("leg-26", "H1", &["A"])];
    for (said, why) in [
        ("Leg 26 is red", "said in prose"),
        ("leg-26 is red", "said as written"),
        ("the leg-26-follow-up is red", "said inside a longer id"),
    ] {
        assert_eq!(taken_by(said, &hubs), ["H1"], "{why}: not taken");
    }
    for (said, why) in [
        ("Leg 27 is red", "another number"),
        ("Leg 27 is red in 26 runs", "its number said apart"),
        ("the leg is red", "no number"),
    ] {
        assert!(taken_by(said, &hubs).is_empty(), "{why}: taken");
    }
}

// The harness's own cases for a subject's word past LONGEST_WORD, from
// its need-route-subjects.test.ts: no need keeps one, so its hub takes
// none, and one as long as the cap is taken said whole.
#[test]
fn a_subject_saying_a_word_past_the_longest_takes_nothing() {
    let (long, longest) = ("p".repeat(33), "p".repeat(32));
    let said = |extra: &str| vec!["easing".to_owned(), extra.to_owned(), "lines".to_owned()];
    let hubs = [Hub::new(&format!("x:{long}-lines"), "H1", &["A"])];
    assert!(
        taking(NEED, &said("other"), &hubs, &|_| true).is_empty(),
        "a hub took a need saying only its subject's shorter words"
    );
    assert!(
        taking(NEED, &said(&long), &hubs, &|_| true).is_empty(),
        "a hub took a need saying a word longer than any a need keeps"
    );
    let hubs = [Hub::new(&format!("x:{longest}-lines"), "H1", &["A"])];
    assert_eq!(
        taking(NEED, &said(&longest), &hubs, &|_| true).len(),
        1,
        "a hub of a word as long as the cap wasn't taken by it said whole"
    );
    let word = "p".repeat(26);
    let hubs = [Hub::new(&format!("x:{word}-leg-26"), "H1", &["A"])];
    assert!(
        taken_by(&format!("easing {word} leg 27"), &hubs).is_empty(),
        "a hub whose numbered id is past the cap took a need for another number by \
         its other words"
    );
}

#[test]
fn each_need_is_kept_once_in_its_days_file() {
    // 01:25:46 on 19 November 2026, UTC.
    let at = 1_795_051_546_000;
    let needs = [
        Need::put("CADENCE", 0, at, vec!["chrome".into()], ("H2", "D"), 130),
        Need::put("CADENCE", 1, at, vec!["cadence".into()], ("D", "D"), 130),
    ];
    assert_eq!(
        needs.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
        ["CADENCE/mwmvcdcg.0", "CADENCE/mwmvcdcg.1"],
        "two needs shared an id, or it isn't the harness's shape"
    );
    assert_eq!(
        (needs[0].path.clone(), needs[1].path.clone()),
        (vec!["H2".to_owned(), "D".to_owned()], vec!["D".to_owned()]),
        "a need's path didn't run through its hub to its holder"
    );
    let root = Folder::new("kept");
    keep(&root.0, &[]).expect("nothing kept");
    assert!(
        !root.0.join("mesh-workflow").exists(),
        "no needs made a folder"
    );
    keep(&root.0, &needs).expect("kept");
    let file = root.0.join("mesh-workflow/needs/2026-11-19.jsonl");
    let kept = std::fs::read_to_string(&file).expect("its day's file");
    let lines: Vec<serde_json::Value> = kept
        .lines()
        .map(|l| serde_json::from_str(l).expect("a line of JSON"))
        .collect();
    assert_eq!(lines.len(), 2, "{kept}");
    assert_eq!(
        lines[0]["need"], "CADENCE/mwmvcdcg.0",
        "its id isn't under need, as the harness reads it: {kept}"
    );
    assert_eq!(
        (&lines[0]["asker"], &lines[0]["fit"], &lines[0]["cells"]),
        (&"CADENCE".into(), &"D".into(), &130.into()),
        "{kept}"
    );
    // A need the harness kept on 19 November, its words cut short.
    let theirs: serde_json::Value = serde_json::from_str(
        r#"{"need":"D46U6P/mwmvunad.34551b9436d4","asker":"D46U6P","words":["shell"],
            "path":["WCJQSG","RCTRF8"],"fit":"RCTRF8","at":1795052398693,"cells":101}"#,
    )
    .expect("the harness's line");
    let keys = |v: &serde_json::Value| {
        v.as_object()
            .map(|o| o.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    };
    assert_eq!(
        keys(&lines[0]),
        keys(&theirs),
        "not the harness's shape: {kept}"
    );
    assert_eq!(base36(0), "0", "nought");
    assert_eq!(base36(36), "10", "a carry");
}

#[test]
fn a_needs_day_is_its_utc_day() {
    assert_eq!(utc_day(0), "1970-01-01", "the epoch");
    assert_eq!(
        utc_day(1_795_051_546_000),
        "2026-11-19",
        "an early hour of 19 November, UTC"
    );
    assert_eq!(
        utc_day(951_782_400_000),
        "2000-02-29",
        "a leap day of a century's year"
    );
    assert_eq!(
        utc_day(-1),
        "1969-12-31",
        "a moment before the epoch is the day before"
    );
}

#[test]
fn the_socket_is_the_realms_named_or_kept() {
    let runtime = Path::new("/run/user/1000");
    let state = Folder::new("socket");
    assert_eq!(
        socket(runtime, &state.0, Some("personal".into())),
        Some(PathBuf::from("/run/user/1000/mycelium/personal.sock")),
        "the realm named"
    );
    assert_eq!(socket(runtime, &state.0, None), None, "no realm at all");
    std::fs::create_dir_all(state.0.join("mycelium")).expect("its folder");
    std::fs::write(state.0.join("mycelium/realm"), "work\n").expect("written");
    assert_eq!(
        socket(runtime, &state.0, Some(String::new())),
        Some(PathBuf::from("/run/user/1000/mycelium/work.sock")),
        "the realm the node keeps"
    );
    assert_eq!(
        socket(runtime, &state.0, Some("../work".into())),
        None,
        "a realm that isn't a plain name"
    );
}

/// A node at a socket in a new folder, answering one asker with
/// `answers`, a line each, after reading its two asks.
fn node(name: &str, answers: &'static [&'static str]) -> (Folder, PathBuf) {
    let folder = Folder::new(name);
    let socket = folder.0.join("work.sock");
    let listener = std::os::unix::net::UnixListener::bind(&socket).expect("bound");
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("asked");
        let mut asks = BufReader::new(stream.try_clone().expect("its other end")).lines();
        let _ = (asks.next(), asks.next());
        let mut stream = stream;
        for answer in answers {
            let _ = writeln!(stream, "{answer}");
        }
    });
    (folder, socket)
}

#[test]
fn the_list_is_read_from_the_line_with_its_id() {
    let (_folder, socket) = node(
        "listed",
        &[
            r#"{"id":1,"hello":"work"}"#,
            r#"{"event":"joined","records":[{"key":"hub:other","hub":"X"}]}"#,
            r#"{"id":2,"records":[{"key":"hub:cadence","hub":"H1","fits":["F1"]}]}"#,
        ],
    );
    assert_eq!(
        listed(&socket).expect("listed"),
        [Hub::new("cadence", "H1", &["F1"])],
        "a line other than the list's answer was read as it"
    );
    let (_folder, socket) = node("unlisted", &[r#"{"id":1,"hello":"work"}"#]);
    assert!(
        listed(&socket).is_err(),
        "a node that never answered the list read as one with no hubs"
    );
    assert!(
        listed(Path::new("/nonexistent/work.sock")).is_err(),
        "no node read as one with no hubs"
    );
}

#[test]
fn a_node_that_never_answers_is_given_up_on_by_the_deadline() {
    let folder = Folder::new("silent");
    let socket = folder.0.join("work.sock");
    // Bound and never accepted, so the connect lands in its backlog and
    // no line ever comes back.
    let _listener = std::os::unix::net::UnixListener::bind(&socket).expect("bound");
    let begun = std::time::Instant::now();
    let error = listed_within(&socket, Duration::from_millis(200)).expect_err("no answer");
    assert_eq!(
        error.kind(),
        io::ErrorKind::TimedOut,
        "a silent node wasn't said as one out of time: {error}"
    );
    assert!(
        begun.elapsed() < Duration::from_secs(2),
        "the ask outlasted its deadline: {:?}",
        begun.elapsed()
    );
}
