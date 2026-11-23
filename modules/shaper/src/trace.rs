//! What the live shaper heard and held each second that the recorder's
//! lines can't say, kept as day files of its own: whose the upload's queue
//! was, a call's round trip through the VPN, the rates its controller
//! asked of the upload and the download, and the ceiling, a trial's, its
//! network's or its house's, the upload was held under.
//! A replay (src/replay.rs)
//! puts these back into the recorded seconds, so a bite that came
//! through our own queues or the VPN is replayed rather than passed as
//! clean, and the rates the live controller asked are scored beside any a
//! replayed one would set.
//!
//! One short line a second, about 20 MB a day, kept for as many days as
//! the recorder keeps.

use crate::oneway::Ours;
use crate::record::{day_file, expired};
use crate::replay::{Rates, Stay};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

/// How long, in seconds, a held rate stands for the seconds after it
/// that have no line, as a step that missed a write leaves them.
const STANDS_S: u64 = 5;

/// One second as the live shaper took it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Line {
    /// The recorded second it was stepped on, in seconds since 1970.
    pub at: u64,
    /// The link it holds.
    pub link: String,
    /// What the controller asked each way be held to, in kbit/s, before the
    /// shaper's floor and the link's most; none asked no limit. A line
    /// written before the download was traced has none for it too, so a
    /// held replay of its day reads the download as never limited; and its
    /// upload is the way's own rate, which the controller doesn't hand over
    /// while it lets go or hears no reflector, so such a replay scores those
    /// spells at a rate that was never asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub down: Option<u64>,
    /// Whose the upload's queue was. A line written before 21 November has
    /// no `agents_straight` in it, so a replay of such a day never takes
    /// the path where agents yield.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ours: Option<Ours>,
    /// A call's round trip through the VPN, and the least it was taken to
    /// be, in ms, to a tenth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vpn_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vpn_base_ms: Option<f64>,
    /// The plateau the upload holds under, in kbit/s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plateau: Option<u64>,
    /// The ceiling the upload was held under, a trial's, its network's or
    /// its house's that second, in kbit/s, while one did: a rate held there
    /// is the ceiling's, not the controller's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ceiling: Option<u64>,
    /// What another machine on the line said it sent, and the most the
    /// house's ceiling left this one, in kbit/s, while it said so lately
    /// (src/house.rs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub house_sent: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub house_most: Option<u64>,
    /// What the download was held to for a call that second, in kbit/s,
    /// while one was (src/room.rs): a hold of the shaper's own, under the
    /// line, apart from the controller's `down`, so a replay doesn't read
    /// it as a cut the controller judged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room: Option<u64>,
    /// The kind of reason the upload held where it did (`Why::kind`), and
    /// the shares the VPN's own queue and agents' lane were held to on the
    /// link, as main put them there, down to a hundredth, while under one:
    /// holds that leave the upload's rate as it was, and that the journal
    /// says only as they begin or end. A line with a `why` was written by a
    /// shaper whose `link_wait_ms` leaves agents' lane out (src/oneway.rs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vpn_share: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_share: Option<f64>,
    /// True where tc refused the split's last change, so the shares are
    /// what main asked, not known to be what the classes hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held_unsent: Option<bool>,
}

impl Line {
    /// The line for what the controller heard of `link` at the recorded
    /// second `at`.
    pub fn of(at: u64, link: &str, heard: &crate::live::Heard) -> Line {
        let tenth = |ms: f64| (ms * 10.0).round() / 10.0;
        Line {
            at,
            link: link.to_owned(),
            up: heard.up_limit,
            down: heard.down_limit,
            ours: heard.ours,
            vpn_ms: heard.vpn_ms.map(tenth),
            vpn_base_ms: heard.vpn_base_ms.map(tenth),
            plateau: heard.plateau.map(crate::rate::whole),
            ceiling: heard.ceiling.map(crate::rate::whole),
            house_sent: None,
            house_most: None,
            room: None,
            why: None,
            vpn_share: None,
            agents_share: None,
            held_unsent: None,
        }
    }

    /// This line, saying the controller's kind of reason (`Why::kind`).
    pub fn because(self, why: &str) -> Line {
        Line {
            why: Some(why.to_owned()),
            ..self
        }
    }

    /// This line, saying the shares its link's queues were held to, those
    /// under one, rounded down to a hundredth (`why::percent`), so a share
    /// held under one never reads as one, and marked where the split they
    /// came from wasn't sent.
    pub fn held(self, held: Held) -> Line {
        let under = |share: f64| (share < 1.0).then(|| crate::why::percent(share) / 100.0);
        Line {
            vpn_share: under(held.vpn),
            agents_share: under(held.agents),
            held_unsent: held.unsent.then_some(true),
            ..self
        }
    }
}

/// The shares a link's queues were held to, as main put them there: the
/// VPN's own queue's of its lane, and agents' lane's of its split, each one
/// while it held nothing; and whether the split's last change failed, so
/// the classes may hold another than these.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Held {
    pub vpn: f64,
    pub agents: f64,
    pub unsent: bool,
}

impl Held {
    /// Nothing held.
    pub const WHOLE: Held = Held {
        vpn: 1.0,
        agents: 1.0,
        unsent: false,
    };
}

/// Writes a line for each recorded second the shaper steps on, once.
pub struct Writer {
    folder: PathBuf,
    last: Option<u64>,
    day: Option<PathBuf>,
}

impl Writer {
    pub fn new(folder: &Path) -> Writer {
        Writer {
            folder: folder.to_path_buf(),
            last: None,
            day: None,
        }
    }

    /// Writes `line` unless its second was written already. A write that
    /// fails is let go: the trace is for a replay later, and the shaping
    /// mustn't wait on a disk.
    pub fn write(&mut self, line: &Line) {
        if self.last.is_some_and(|last| line.at <= last) {
            return;
        }
        self.last = Some(line.at);
        let path = day_file(&self.folder, UNIX_EPOCH + Duration::from_secs(line.at));
        if self.day.as_ref() != Some(&path) {
            let _ = fs::create_dir_all(&self.folder);
            self.tidy();
            self.day = Some(path.clone());
        }
        let text = serde_json::to_string(line).expect("a trace line serialises");
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(file, "{text}");
        }
    }

    fn tidy(&self) {
        let names: Vec<String> = fs::read_dir(&self.folder)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        for name in expired(&names) {
            let _ = fs::remove_file(self.folder.join(name));
        }
    }
}

/// Every line in a folder's day files, by its second. A line that can't
/// be read is passed over, as the recorder's are.
pub fn read(folder: &Path) -> Result<BTreeMap<u64, Line>, String> {
    let entries = fs::read_dir(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let mut lines = BTreeMap::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "jsonl") {
            continue;
        }
        let file = File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        for text in BufReader::new(file).lines().map_while(Result::ok) {
            if let Ok(line) = serde_json::from_str::<Line>(&text) {
                lines.insert(line.at, line);
            }
        }
    }
    Ok(lines)
}

/// Puts what the shaper heard back into each stay's seconds, and gives
/// the rates it held at each, in the stays' order: a second with no line
/// holds the last rates, and the ceiling over the upload, for up to
/// `STANDS_S`, and no limit after.
pub fn merge(stays: &mut [Stay], lines: &BTreeMap<u64, Line>) -> Vec<Vec<Rates>> {
    stays
        .iter_mut()
        .map(|stay| {
            let began = stay.began_ms / 1_000;
            let mut last: Option<(u64, &Line)> = None;
            stay.seconds
                .iter_mut()
                .map(|second| {
                    let at = began + crate::rate::whole(second.t_s);
                    let held = if let Some(line) = lines.get(&at) {
                        second.up_ours = line.ours;
                        second.vpn_ms = line.vpn_ms;
                        second.ceiling = line.ceiling;
                        last = Some((at, line));
                        Some(line)
                    } else {
                        let standing = last.filter(|(then, _)| at - then <= STANDS_S);
                        // A step that missed a write held the same trial,
                        // so its seconds are the trial's too.
                        second.ceiling = standing.and_then(|(_, l)| l.ceiling);
                        standing.map(|(_, l)| l)
                    };
                    Rates {
                        up: held.and_then(|l| l.up),
                        down: held.and_then(|l| l.down),
                    }
                })
                .collect()
        })
        .collect()
}

/// Puts the trace's `lines` into `stays` as `merge` does, giving the
/// rates the live shaper held. Replaying that shaper, `held`, only what
/// the trace saw is kept, as `traced` says; replaying another
/// controller, every ceiling the trace kept is forgotten, since it held
/// only the live shaper's rates; that replay sets its own (`--most`).
pub fn heard_into(
    mut stays: Vec<Stay>,
    lines: &BTreeMap<u64, Line>,
    held: bool,
) -> (Vec<Stay>, Vec<Vec<Rates>>) {
    let rates = merge(&mut stays, lines);
    if held {
        return traced(stays, rates, lines);
    }
    forget_trials(&mut stays);
    (stays, rates)
}

/// Forgets the ceilings `merge` put in.
fn forget_trials(stays: &mut [Stay]) {
    for second in stays.iter_mut().flat_map(|s| s.seconds.iter_mut()) {
        second.ceiling = None;
    }
}

/// Keeps, of each stay and the rates `merge` gave it, only the stretch
/// from its first second the trace has a line for to its last, since a
/// second before the trace began reads as no limit and would be judged
/// as never held low; a stay the trace never saw is left out.
pub fn traced(
    stays: Vec<Stay>,
    held: Vec<Vec<Rates>>,
    lines: &BTreeMap<u64, Line>,
) -> (Vec<Stay>, Vec<Vec<Rates>>) {
    stays
        .into_iter()
        .zip(held)
        .filter_map(|(stay, rates)| {
            let began = stay.began_ms / 1_000;
            let at = |s: &crate::replay::Second| began + crate::rate::whole(s.t_s);
            let seen = |s: &crate::replay::Second| lines.contains_key(&at(s));
            let first = stay.seconds.iter().position(seen)?;
            let last = stay.seconds.iter().rposition(seen)?;
            let from = crate::rate::whole(stay.seconds[first].t_s);
            let seconds = stay.seconds[first..=last]
                .iter()
                .map(|s| crate::replay::Second {
                    t_s: s.t_s - crate::rate::float(from),
                    ..s.clone()
                })
                .collect();
            let kept = Stay {
                began_ms: (began + from) * 1_000,
                seconds,
                ..stay
            };
            Some((kept, rates[first..=last].to_vec()))
        })
        .unzip()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::Second;

    /// Midnight in UTC on 16 November 2026, in seconds.
    const NOV16: u64 = 1_794_787_200;

    fn second(t: u64) -> Second {
        Second {
            t_s: crate::rate::float(t),
            up_kbit: 1_000.0,
            down_kbit: 0.0,
            up_delay_ms: Some(1.0),
            down_delay_ms: None,
            gateway_ms: None,
            gateway_most_ms: None,
            up_ours: None,
            vpn_ms: None,
            ceiling: None,
        }
    }

    fn line(at: u64, up: Option<u64>, vpn_ms: Option<f64>) -> Line {
        Line {
            at,
            link: "wlan0".into(),
            up,
            ours: Some(Ours {
                queued: true,
                ..Ours::default()
            }),
            vpn_ms,
            ..Line::default()
        }
    }

    #[test]
    fn a_line_says_the_ceiling_the_upload_is_held_under() {
        let heard = |ceiling| crate::live::Heard {
            up_limit: Some(9_500),
            ceiling,
            ..crate::live::Heard::default()
        };
        let at = |h: &crate::live::Heard| Line::of(NOV16, "wlan0", h).ceiling;
        assert_eq!(at(&heard(Some(9_500.4))), Some(9_500), "a ceiling");
        assert_eq!(at(&heard(None)), None, "no ceiling");
        // Written only when there is one, so a day's lines stay short.
        let line = serde_json::to_string(&Line::of(NOV16, "wlan0", &heard(None)));
        assert!(
            !line.expect("a line").contains("ceiling"),
            "wrote no ceiling"
        );
    }

    #[test]
    fn a_line_says_why_it_held_and_the_holds_under_its_rate() {
        let held = line(NOV16, Some(9_500), None).because("tunnel").held(Held {
            vpn: 0.8149,
            agents: 0.996,
            ..Held::WHOLE
        });
        assert_eq!(held.why.as_deref(), Some("tunnel"), "the kind of reason");
        assert_eq!(
            held.vpn_share,
            Some(0.81),
            "the VPN's own queue's share, to a hundredth"
        );
        assert_eq!(
            held.agents_share,
            Some(0.99),
            "agents' share, rounded down under one"
        );
        let edge = line(NOV16, None, None).held(Held {
            vpn: 0.29,
            agents: 1.0,
            ..Held::WHOLE
        });
        assert_eq!(
            edge.vpn_share,
            Some(0.29),
            "a share a double keeps a hair under its hundredth reads as that hundredth"
        );
        let free = line(NOV16, Some(9_500), None)
            .because("clear")
            .held(Held::WHOLE);
        assert_eq!(
            (free.vpn_share, free.agents_share),
            (None, None),
            "no share at one"
        );
        let written = serde_json::to_string(&free).expect("a line");
        assert!(
            written.contains(r#""why":"clear""#) && !written.contains("share"),
            "wrote the reason alone: {written}"
        );
        assert_eq!(
            held.held_unsent, None,
            "shares from a split sent were marked unsent"
        );
        // A split tc refused leaves the shares what main asked, which the
        // line says (traces round 2's F13).
        let refused = line(NOV16, Some(9_500), None).held(Held {
            vpn: 0.5,
            agents: 1.0,
            unsent: true,
        });
        let written = serde_json::to_string(&refused).expect("a line");
        assert!(
            written.contains(r#""held_unsent":true"#),
            "shares from a split tc refused weren't marked: {written}"
        );
    }

    #[test]
    fn a_trace_is_written_once_a_second_and_read_back() {
        let folder = std::env::temp_dir().join(format!("shaper-trace-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        let mut writer = Writer::new(&folder);
        writer.write(&Line {
            down: Some(20_000),
            ..line(NOV16 + 5, Some(10_800), Some(87.0))
        });
        writer.write(&line(NOV16 + 5, Some(1), None));
        writer.write(&line(NOV16 + 6, None, None).because("tunnel").held(Held {
            vpn: 0.5,
            agents: 1.0,
            ..Held::WHOLE
        }));
        let read = read(&folder).expect("the folder reads");
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(
            read.len(),
            2,
            "a second written twice is kept once: {read:?}"
        );
        assert_eq!(
            read[&(NOV16 + 5)].up,
            Some(10_800),
            "the first write stands: {read:?}"
        );
        assert_eq!(
            read[&(NOV16 + 5)].vpn_ms,
            Some(87.0),
            "with the VPN's: {read:?}"
        );
        assert_eq!(
            read[&(NOV16 + 6)].up,
            None,
            "no limit reads back as none: {read:?}"
        );
        assert_eq!(
            (read[&(NOV16 + 5)].down, read[&(NOV16 + 6)].down),
            (Some(20_000), None),
            "the download's ask, and none: {read:?}"
        );
        assert_eq!(
            (
                read[&(NOV16 + 6)].why.as_deref(),
                read[&(NOV16 + 6)].vpn_share,
                read[&(NOV16 + 6)].agents_share
            ),
            (Some("tunnel"), Some(0.5), None),
            "the reason and the shares under one read back: {read:?}"
        );
        let before: Line = serde_json::from_str(r#"{"at":1,"link":"wlan0","up":5000}"#)
            .expect("a line from before the download was traced reads");
        assert_eq!((before.up, before.down), (Some(5_000), None));
    }

    #[test]
    fn a_stays_seconds_take_what_the_shaper_heard_and_held() {
        let mut stays = vec![Stay {
            network: "home".into(),
            began_ms: NOV16 * 1_000,
            seconds: (0..12).map(second).collect(),
        }];
        let lines = BTreeMap::from([
            (NOV16 + 1, line(NOV16 + 1, Some(5_000), Some(120.0))),
            (
                NOV16 + 2,
                Line {
                    ceiling: Some(4_800),
                    ..line(NOV16 + 2, Some(4_800), None)
                },
            ),
        ]);
        let held = merge(&mut stays, &lines);
        let ups: Vec<Option<u64>> = held[0].iter().map(|r| r.up).collect();
        let mut want = vec![None, Some(5_000)];
        want.extend([Some(4_800); 6]);
        want.extend([None; 4]);
        assert_eq!(ups, want, "held, then standing 5 s past the last line");
        let s = &stays[0].seconds;
        assert_eq!(s[1].vpn_ms, Some(120.0), "the VPN's round trip: {:?}", s[1]);
        assert!(
            s[2].up_ours.is_some_and(|o| o.queued),
            "our queue: {:?}",
            s[2]
        );
        assert_eq!(s[2].ceiling, Some(4_800), "a trial's ceiling: {:?}", s[2]);
        assert_eq!(s[1].ceiling, None, "none, with no trial: {:?}", s[1]);
        let ceilings: Vec<Option<u64>> = s.iter().map(|s| s.ceiling).collect();
        let mut want = vec![None, None];
        want.extend([Some(4_800); 6]);
        want.extend([None; 4]);
        assert_eq!(ceilings, want, "the trial standing with its rate");
        assert_eq!(
            s[3].up_ours, None,
            "nothing heard, nothing put in: {:?}",
            s[3]
        );
    }

    #[test]
    fn a_stays_seconds_hold_the_download_the_controller_asked() {
        let heard = |down_limit| crate::live::Heard {
            down_limit,
            ..crate::live::Heard::default()
        };
        let at = |h: &crate::live::Heard| Line::of(NOV16, "wlan0", h).down;
        assert_eq!(at(&heard(Some(20_000))), Some(20_000), "a limit");
        assert_eq!(at(&heard(None)), None, "no limit asked");
        let mut stays = vec![Stay {
            network: "home".into(),
            began_ms: NOV16 * 1_000,
            seconds: (0..10).map(second).collect(),
        }];
        let lines = BTreeMap::from([
            (
                NOV16 + 1,
                Line {
                    down: Some(20_000),
                    ..line(NOV16 + 1, Some(5_000), None)
                },
            ),
            (NOV16 + 2, line(NOV16 + 2, Some(5_000), None)),
            (
                NOV16 + 3,
                Line {
                    down: Some(8_000),
                    ..line(NOV16 + 3, Some(5_000), None)
                },
            ),
        ]);
        let held = merge(&mut stays, &lines);
        let downs: Vec<Option<u64>> = held[0].iter().map(|r| r.down).collect();
        let mut want = vec![None, Some(20_000), None];
        want.extend([Some(8_000); 6]);
        want.push(None);
        assert_eq!(
            downs, want,
            "asked, no limit asked, then standing 5 s past the last line"
        );
    }

    #[test]
    fn a_trial_s_ceiling_is_kept_only_for_the_live_shaper() {
        let stays = || {
            vec![Stay {
                network: "home".into(),
                began_ms: NOV16 * 1_000,
                seconds: (0..4).map(second).collect(),
            }]
        };
        let lines = BTreeMap::from([(
            NOV16 + 1,
            Line {
                ceiling: Some(8_000),
                ..line(NOV16 + 1, Some(8_000), None)
            },
        )]);
        let ceilings = |held| {
            let (stays, _) = heard_into(stays(), &lines, held);
            stays[0]
                .seconds
                .iter()
                .map(|s| s.ceiling)
                .collect::<Vec<_>>()
        };
        // Replaying the live shaper, from the trace's first line on.
        assert_eq!(ceilings(true), [Some(8_000)], "the live shaper's trial");
        assert_eq!(
            ceilings(false),
            [None; 4],
            "a replayed controller held under the live one's trial"
        );
    }

    #[test]
    fn only_the_seconds_the_trace_covers_are_judged_as_held() {
        let mut stays = vec![
            Stay {
                network: "home".into(),
                began_ms: NOV16 * 1_000,
                seconds: (0..12).map(second).collect(),
            },
            Stay {
                network: "cafe".into(),
                began_ms: (NOV16 + 100) * 1_000,
                seconds: (0..5).map(second).collect(),
            },
        ];
        let lines = BTreeMap::from([
            (NOV16 + 3, line(NOV16 + 3, Some(5_000), None)),
            (NOV16 + 7, line(NOV16 + 7, None, None)),
        ]);
        let held = merge(&mut stays, &lines);
        let (stays, held) = traced(stays, held, &lines);
        assert_eq!(stays.len(), 1, "a stay the trace never saw is left out");
        assert_eq!(
            stays[0].began_ms,
            (NOV16 + 3) * 1_000,
            "it begins at its first traced second"
        );
        let starts: Vec<u64> = stays[0]
            .seconds
            .iter()
            .map(|s| crate::rate::whole(s.t_s))
            .collect();
        assert_eq!(starts, vec![0, 1, 2, 3, 4], "and ends at its last");
        let ups: Vec<Option<u64>> = held[0].iter().map(|r| r.up).collect();
        assert_eq!(
            ups,
            vec![Some(5_000), Some(5_000), Some(5_000), Some(5_000), None],
            "each second keeps the rate it was held at"
        );
    }
}
