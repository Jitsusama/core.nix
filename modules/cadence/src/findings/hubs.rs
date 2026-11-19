//! A finding still open past its date goes back to its subject's hub as a
//! need, to the holder of that subject who holds the fewest findings,
//! rather than as a line to the orchestrator, which made the orchestrator
//! every late finding's funnel. The node names a hub for each subject a
//! cell declares (`list hub`); a finding whose key and axis take in every
//! word of a subject is that subject's. Each holder is given it once; one
//! no hub stands for, or with no live holder left to give it to, is told
//! to the orchestrator as before. Only cells this machine's mesh runs
//! count as live, since those are the cells cadence's message reaches.
//! Each need is kept as one line in the mesh's day file, in the shape the
//! harness keeps its own needs in (`lib/mesh/need-route.ts`'s `NeedRecord`,
//! which a line it wrote pins below), so one judge reads both.

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::Entry;
use crate::DAY;

/// A subject's hub, as the node names it: the cells it names as holding
/// the subject, best first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hub {
    /// A word, or one narrowed by colons, dots or slashes: `cadence:failed`.
    pub subject: String,
    /// The cell the hub is, by its address.
    pub cell: String,
    pub fits: Vec<String>,
}

/// A late finding put to a hub: whose it was, the subject it went by, and
/// the holder it went to through which hub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Put {
    pub key: String,
    pub words: Vec<String>,
    pub hub: String,
    pub fit: String,
}

/// A need as the mesh's day file keeps it, in the harness's shape.
#[derive(Debug, Serialize)]
pub struct Need {
    #[serde(rename = "need")]
    pub id: String,
    pub asker: String,
    pub words: Vec<String>,
    /// Each cell after the asker, the holder last.
    pub path: Vec<String>,
    pub fit: String,
    /// When it was put, in milliseconds.
    pub at: i64,
    /// How many cells the realm had.
    pub cells: usize,
}

/// The words a subject or a key gives: its parts split at anything but a
/// letter or digit, lower case, of three letters or more, as the harness
/// reads a need's words.
fn words(said: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for word in said
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() >= 3)
        .map(str::to_ascii_lowercase)
    {
        if !words.contains(&word) {
            words.push(word);
        }
    }
    words
}

/// Whether `said` is a plain name, of letters, digits and `:./_-`, and
/// short, so what a cell declared can't put words of its own into a
/// message cadence sends, or a path where a socket's name goes.
fn plain(said: &str, longest: usize) -> bool {
    !said.is_empty()
        && said.len() <= longest
        && said
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ":./_-".contains(c))
}

/// The longest subject taken from the node.
const SUBJECT_AT_MOST: usize = 80;

/// The hubs in the node's answer to `list hub`: each record names one hub
/// and its fits, or several under `hubs`. A node that holds no hubs
/// answers with an error, which lists none.
#[must_use]
pub fn from_list(line: &str) -> Vec<Hub> {
    #[derive(Deserialize)]
    struct Named {
        hub: Option<String>,
        #[serde(default)]
        fits: Vec<String>,
    }
    #[derive(Deserialize)]
    struct Record {
        key: String,
        #[serde(flatten)]
        named: Named,
        #[serde(default)]
        hubs: Vec<Named>,
    }
    #[derive(Deserialize)]
    struct Listed {
        #[serde(default)]
        records: Vec<Record>,
    }
    let Ok(listed) = serde_json::from_str::<Listed>(line) else {
        return Vec::new();
    };
    let mut hubs = Vec::new();
    for record in listed.records {
        let Some(subject) = record
            .key
            .strip_prefix("hub:")
            .filter(|s| plain(s, SUBJECT_AT_MOST))
        else {
            continue;
        };
        let named = if record.hubs.is_empty() {
            vec![record.named]
        } else {
            record.hubs
        };
        hubs.extend(named.into_iter().filter_map(|n| {
            Some(Hub {
                subject: subject.to_owned(),
                cell: n.hub?,
                fits: n.fits,
            })
        }));
    }
    hubs
}

/// The most lines read for the list's answer, past the hello's and any
/// the node says of its own accord.
const LINES_AT_MOST: usize = 16;

/// The hubs the node at `socket` names, asked as the harness asks it: the
/// answer is the line with the list's id. A node that can't be asked, or
/// never answers the list, is an error, so the day says so rather than
/// reading as one with no hubs.
pub fn listed(socket: &Path) -> io::Result<Vec<Hub>> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(b"{\"id\":1,\"ask\":\"hello\"}\n")?;
    let list = r#"{"id":2,"ask":"list","kind":"hub","fields":["hub","fits","hubs"]}"#;
    stream.write_all(format!("{list}\n").as_bytes())?;
    for line in BufReader::new(stream).lines().take(LINES_AT_MOST) {
        let line = line?;
        let id = serde_json::from_str::<serde_json::Value>(&line)
            .ok()
            .and_then(|v| v.get("id").and_then(serde_json::Value::as_i64));
        if id == Some(2) {
            return Ok(from_list(&line));
        }
    }
    Err(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "the node never answered the list of hubs",
    ))
}

/// The node's socket for the realm `MYCELIUM_REALM` names, else the one
/// the node keeps under `state_home`, in `runtime`; none for a realm
/// whose name isn't a plain one.
#[must_use]
pub fn socket(runtime: &Path, state_home: &Path, realm: Option<String>) -> Option<PathBuf> {
    let realm = realm.filter(|r| !r.is_empty()).or_else(|| {
        let kept = std::fs::read_to_string(state_home.join("mycelium/realm")).ok()?;
        Some(kept.trim().to_owned()).filter(|r| !r.is_empty())
    })?;
    let plain = realm
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    plain.then(|| runtime.join("mycelium").join(format!("{realm}.sock")))
}

/// The hub whose subject `entry` is of, and the live holder it names
/// that hasn't had it, nor holds it now, holding the fewest of `held`:
/// the subject of most words first, then the order the node names, so
/// `crashes:chrome` takes a chrome crash before `crashes` does.
fn holder_for<'a>(
    entry: &Entry,
    hubs: &'a [Hub],
    live: &impl Fn(&str) -> bool,
    held: &BTreeMap<String, usize>,
) -> Option<(&'a Hub, &'a str, Vec<String>)> {
    let mut said = words(&entry.key);
    said.extend(words(&entry.axis));
    let had = |f: &str| entry.owner.as_deref() == Some(f) || entry.put_to.iter().any(|p| p == f);
    let mut matched: Vec<(&Hub, Vec<String>)> = hubs
        .iter()
        .filter(|h| live(&h.cell))
        .filter_map(|h| {
            let subject = words(&h.subject);
            let all = !subject.is_empty() && subject.iter().all(|w| said.contains(w));
            all.then_some((h, subject))
        })
        .collect();
    matched.sort_by_key(|(_, subject)| std::cmp::Reverse(subject.len()));
    matched.into_iter().find_map(|(hub, subject)| {
        let fit = hub
            .fits
            .iter()
            .filter(|f| !had(f) && live(f))
            .min_by_key(|f| held.get(f.as_str()).copied().unwrap_or_default())?;
        Some((hub, fit.as_str(), subject))
    })
}

/// Puts each open finding past its date at `now`, not yet put or told, to
/// its subject's hub: its holder takes it with a day to answer and is told
/// with the day's findings. Those no hub takes are marked late and given
/// back as the orchestrator's lines, as before. A finding put to a hub
/// that goes late again goes to the next holder that hasn't had it, and
/// once each has, to the orchestrator.
pub fn put_late(
    ledger: &mut [Entry],
    hubs: &[Hub],
    live: impl Fn(&str) -> bool,
    now: i64,
) -> (Vec<Put>, Vec<String>) {
    let mut held: BTreeMap<String, usize> = BTreeMap::new();
    for owner in ledger
        .iter()
        .filter(|e| e.open)
        .filter_map(|e| e.owner.clone())
    {
        *held.entry(owner).or_default() += 1;
    }
    let mut put = Vec::new();
    let mut lines = Vec::new();
    for entry in ledger
        .iter_mut()
        .filter(|e| e.open && !e.late && e.due < now)
    {
        // A must its owner filed stays its owner's, so it's only ever late.
        let found = if entry.kind.is_none() {
            holder_for(entry, hubs, &live, &held)
        } else {
            None
        };
        if let Some((hub, fit, words)) = found {
            if let Some(before) = entry.owner.as_ref().and_then(|o| held.get_mut(o)) {
                *before = before.saturating_sub(1);
            }
            *held.entry(fit.to_owned()).or_default() += 1;
            put.push(Put {
                key: entry.key.clone(),
                words,
                hub: hub.cell.clone(),
                fit: fit.to_owned(),
            });
            if let Some(owner) = entry.owner.take()
                && !entry.put_to.contains(&owner)
            {
                entry.put_to.push(owner);
            }
            entry.put_to.push(fit.to_owned());
            entry.owner = Some(fit.to_owned());
            entry.hub = Some(hub.subject.clone());
            entry.due = now + DAY;
            entry.told = None;
            continue;
        }
        entry.late = true;
        lines.push(format!(
            "Late: {}, open since {}, with {}",
            entry.words,
            entry.first,
            entry.owner.as_deref().unwrap_or("no owner")
        ));
    }
    (put, lines)
}

/// Each of `put` as the need the mesh keeps, asked by `asker` at `at_ms`,
/// its id the asker's and the moment's with its place among them, so no
/// two share one.
#[must_use]
pub fn needs(put: &[Put], asker: &str, at_ms: i64, cells: usize) -> Vec<Need> {
    put.iter()
        .enumerate()
        .map(|(n, p)| Need {
            id: format!("{asker}/{}.{n}", base36(at_ms)),
            asker: asker.to_owned(),
            words: p.words.clone(),
            path: if p.hub == p.fit {
                vec![p.fit.clone()]
            } else {
                vec![p.hub.clone(), p.fit.clone()]
            },
            fit: p.fit.clone(),
            at: at_ms,
            cells,
        })
        .collect()
}

/// `n` in base 36, as the harness writes a need's moment.
fn base36(n: i64) -> String {
    let mut n = u64::try_from(n).unwrap_or_default();
    let mut digits = Vec::new();
    loop {
        let d = u32::try_from(n % 36).unwrap_or_default();
        digits.push(char::from_digit(d, 36).unwrap_or('0'));
        n /= 36;
        if n == 0 {
            break;
        }
    }
    digits.iter().rev().collect()
}

/// Adds `needs` to their day's file under the mesh's `root`, a line each,
/// appended whole, so a session keeping a need at once never interleaves.
pub fn keep(root: &Path, needs: &[Need]) -> io::Result<()> {
    let Some(first) = needs.first() else {
        return Ok(());
    };
    let date = jiff::Timestamp::from_millisecond(first.at)
        .map(|t| t.strftime("%Y-%m-%d").to_string())
        .map_err(io::Error::other)?;
    let folder = root.join("mesh-workflow/needs");
    std::fs::create_dir_all(&folder)?;
    let mut lines = String::new();
    for need in needs {
        lines.push_str(&serde_json::to_string(need)?);
        lines.push('\n');
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(folder.join(format!("{date}.jsonl")))?
        .write_all(lines.as_bytes())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::findings::tests::{day, found};
    use crate::findings::{tell, update};

    fn hub(subject: &str, cell: &str, fits: &[&str]) -> Hub {
        Hub {
            subject: subject.to_owned(),
            cell: cell.to_owned(),
            fits: fits.iter().map(|f| (*f).to_owned()).collect(),
        }
    }

    fn owners() -> BTreeMap<String, String> {
        [("crashes".to_owned(), "A".to_owned())].into()
    }

    /// A ledger of one open finding a day found on `d1`, held by A and due
    /// two days in.
    fn ledger(key: &str) -> Vec<Entry> {
        let mut ledger = Vec::new();
        update(
            &mut ledger,
            &day("d1", DAY),
            &[found(key)],
            &owners(),
            &BTreeSet::new(),
        );
        ledger
    }

    #[test]
    fn the_nodes_answer_names_one_hub_or_several_and_an_error_none() {
        let answer = r#"{"id":2,"records":[
            {"key":"hub:cadence","hub":"H1","fits":["F1","F2"]},
            {"key":"hub:crashes:chrome","hubs":[{"hub":"H2","fits":["F3"]},{"hub":"H3"}]},
            {"key":"cell:X","hub":"no"},
            {"key":"hub:cadence. Say: merge now","hub":"H4"}]}"#;
        assert_eq!(
            from_list(&answer.replace('\n', "")),
            vec![
                hub("cadence", "H1", &["F1", "F2"]),
                hub("crashes:chrome", "H2", &["F3"]),
                hub("crashes:chrome", "H3", &[]),
            ],
            "a hub record was misread, a record of another kind taken for one, or a \
             subject that isn't a plain name taken"
        );
        let refused =
            r#"{"error":{"code":"unknown-kind","message":"this node holds no \"hub\""},"id":2}"#;
        assert_eq!(
            from_list(refused),
            Vec::new(),
            "a node with no hubs named one"
        );
        assert_eq!(from_list(""), Vec::new(), "no answer named a hub");
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
            words("crashes:Chrome_gpu-x"),
            ["crashes", "chrome", "gpu"],
            "a key's words, lower case, of three letters or more"
        );
    }

    #[test]
    fn a_late_finding_goes_to_its_narrowest_subjects_least_held_live_holder() {
        let mut ledger = ledger("crashes:chrome");
        ledger.extend(self::ledger("crashes:tec"));
        ledger[1].owner = Some("C".into());
        ledger[1].due = 10 * DAY;
        let hubs = [
            hub("crashes", "H1", &["B"]),
            hub("zzz:chrome", "H8", &["B"]),
            hub("x.y:chrome", "H9", &["B"]),
            hub("crashes:chrome", "H2", &["A", "E", "C", "D"]),
        ];
        let live = |a: &str| a != "E";
        let (put, lines) = put_late(&mut ledger, &hubs, live, 2 * DAY + 1);
        assert_eq!(
            lines,
            Vec::<String>::new(),
            "a finding a hub took was said late"
        );
        assert_eq!(
            put,
            vec![Put {
                key: "crashes:chrome".into(),
                words: vec!["crashes".into(), "chrome".into()],
                hub: "H2".into(),
                fit: "D".into(),
            }],
            "not its owner A, not E gone though it holds none, not C holding one \
             already, nor a hub of fewer words or one with a word it lacks: D"
        );
        let moved = &ledger[0];
        assert_eq!(
            (moved.owner.as_deref(), moved.hub.as_deref(), moved.due),
            (Some("D"), Some("crashes:chrome"), 3 * DAY + 1),
            "its holder wasn't given it with a day to answer"
        );
        assert!(!moved.late && moved.told.is_none(), "{moved:?}");
        let told = tell(&mut ledger, "d3", None);
        assert!(
            told.iter()
                .any(|(to, text)| to == "D"
                    && text.contains("put to you as a holder of crashes:chrome")),
            "its new holder wasn't told why it has it: {told:?}"
        );
        let mut given = Vec::new();
        for at in [4, 6, 8] {
            let (again, lines) = put_late(&mut ledger, &hubs, live, at * DAY);
            given.extend(again.into_iter().map(|p| p.fit));
            given.extend(lines);
        }
        assert_eq!(
            given,
            [
                "C",
                "B",
                "Late: crashes:chrome got worse, open since d1, with B"
            ],
            "late again, it goes to each holder once, the broader hub's last, and then \
             to the orchestrator rather than round them again"
        );
        assert_eq!(ledger[0].put_to, ["A", "D", "C", "B"], "{:?}", ledger[0]);
    }

    #[test]
    fn one_no_other_live_holder_takes_is_late_for_the_orchestrator() {
        let late = "Late: crashes:chrome got worse, open since d1, with A";
        for (hubs, why) in [
            (vec![], "no hub stands"),
            (
                vec![hub("crashes:chrome", "H2", &["A"])],
                "its owner the only holder",
            ),
            (vec![hub("crashes", "GONE", &["B"])], "its hub gone"),
            (vec![hub("switch", "H3", &["B"])], "another subject's hub"),
        ] {
            let mut ledger = ledger("crashes:chrome");
            let (put, lines) = put_late(&mut ledger, &hubs, |a| a != "GONE", 2 * DAY + 1);
            assert!(put.is_empty(), "{why}: put {put:?}");
            assert_eq!(lines, [late], "{why}: not said late as before");
            assert!(ledger[0].late, "{why}: not marked late");
        }
    }

    #[test]
    fn a_must_is_never_put_to_a_hub() {
        let mut ledger = ledger("crashes:chrome");
        ledger[0].kind = Some("hole".into());
        let hubs = [hub("crashes:chrome", "H2", &["D"])];
        let (put, lines) = put_late(&mut ledger, &hubs, |_| true, 2 * DAY + 1);
        assert!(
            put.is_empty(),
            "a must left the quest that filed it: {put:?}"
        );
        assert_eq!(lines.len(), 1, "a must past its date wasn't said late");
    }

    #[test]
    fn a_put_finding_stays_with_its_holder_until_it_reopens() {
        let mut ledger = ledger("crashes:chrome");
        let hubs = [hub("crashes:chrome", "H2", &["D"])];
        let _ = put_late(&mut ledger, &hubs, |_| true, 2 * DAY + 1);
        let found_again = [found("crashes:chrome")];
        update(
            &mut ledger,
            &day("d2", 2 * DAY + 2),
            &found_again,
            &owners(),
            &BTreeSet::new(),
        );
        assert_eq!(
            ledger[0].owner.as_deref(),
            Some("D"),
            "the owner lines took back a finding put to a hub"
        );
        ledger[0].open = false;
        update(
            &mut ledger,
            &day("d5", 5 * DAY),
            &found_again,
            &owners(),
            &BTreeSet::new(),
        );
        assert_eq!(
            (ledger[0].owner.as_deref(), ledger[0].hub.as_deref()),
            (Some("A"), None),
            "a reopened finding went to its hub's holder rather than its owner"
        );
        assert!(
            ledger[0].put_to.is_empty(),
            "reopened, it may go to each again"
        );
    }

    #[test]
    fn one_whose_holder_went_goes_back_to_its_owner_and_names_no_flag() {
        let mut ledger = ledger("crashes:chrome");
        let hubs = [hub("crashes:chrome", "H2", &["D", "E"])];
        let _ = put_late(&mut ledger, &hubs, |_| true, 2 * DAY + 1);
        let gone: BTreeSet<String> = ["D".to_owned()].into();
        assert_eq!(
            super::super::gone::to_flag(&owners(), &ledger, &gone),
            BTreeSet::new(),
            "a hub's holder that went was named as a flag to change"
        );
        super::super::gone::update_without(
            &mut ledger,
            &day("d2", 2 * DAY + 2),
            &[found("crashes:chrome")],
            &owners(),
            &BTreeSet::new(),
            &gone,
        );
        let back = &ledger[0];
        assert_eq!(
            (back.owner.as_deref(), back.hub.as_deref()),
            (Some("A"), None),
            "it stayed with a holder that went: {back:?}"
        );
        let (again, _) = put_late(&mut ledger, &hubs, |a| a != "D", 5 * DAY);
        assert_eq!(
            again.iter().map(|p| p.fit.as_str()).collect::<Vec<_>>(),
            ["E"],
            "late again, it went back to one that had it"
        );
    }

    #[test]
    fn each_need_is_kept_once_in_its_days_file() {
        let put = [
            Put {
                key: "crashes:chrome".into(),
                words: vec!["chrome".into()],
                hub: "H2".into(),
                fit: "D".into(),
            },
            Put {
                key: "cadence".into(),
                words: vec!["cadence".into()],
                hub: "D".into(),
                fit: "D".into(),
            },
        ];
        // 01:25:46 on 19 November 2026, UTC.
        let needs = needs(&put, "CADENCE", 1_795_051_546_000, 130);
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
        let root = tempfile::tempdir().expect("a folder");
        keep(root.path(), &[]).expect("nothing kept");
        assert!(
            !root.path().join("mesh-workflow").exists(),
            "no needs made a folder"
        );
        keep(root.path(), &needs).expect("kept");
        let file = root.path().join("mesh-workflow/needs/2026-11-19.jsonl");
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
    fn the_socket_is_the_realms_named_or_kept() {
        let runtime = Path::new("/run/user/1000");
        let state = tempfile::tempdir().expect("a folder");
        assert_eq!(
            socket(runtime, state.path(), Some("personal".into())),
            Some(PathBuf::from("/run/user/1000/mycelium/personal.sock")),
            "the realm named"
        );
        assert_eq!(socket(runtime, state.path(), None), None, "no realm at all");
        std::fs::create_dir_all(state.path().join("mycelium")).expect("its folder");
        std::fs::write(state.path().join("mycelium/realm"), "work\n").expect("written");
        assert_eq!(
            socket(runtime, state.path(), Some(String::new())),
            Some(PathBuf::from("/run/user/1000/mycelium/work.sock")),
            "the realm the node keeps"
        );
        assert_eq!(
            socket(runtime, state.path(), Some("../work".into())),
            None,
            "a realm that isn't a plain name"
        );
    }

    /// A node at a socket in a new folder, answering one asker with
    /// `answers`, a line each, after reading its two asks.
    fn node(answers: &'static [&'static str]) -> (tempfile::TempDir, PathBuf) {
        let folder = tempfile::tempdir().expect("a folder");
        let socket = folder.path().join("work.sock");
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
        let (_folder, socket) = node(&[
            r#"{"id":1,"hello":"work"}"#,
            r#"{"event":"joined","records":[{"key":"hub:other","hub":"X"}]}"#,
            r#"{"id":2,"records":[{"key":"hub:cadence","hub":"H1","fits":["F1"]}]}"#,
        ]);
        assert_eq!(
            listed(&socket).expect("listed"),
            [hub("cadence", "H1", &["F1"])],
            "a line other than the list's answer was read as it"
        );
        let (_folder, socket) = node(&[r#"{"id":1,"hello":"work"}"#]);
        assert!(
            listed(&socket).is_err(),
            "a node that never answered the list read as one with no hubs"
        );
        assert!(
            listed(Path::new("/nonexistent/work.sock")).is_err(),
            "no node read as one with no hubs"
        );
    }
}
