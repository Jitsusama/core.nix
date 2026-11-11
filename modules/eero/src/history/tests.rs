use super::*;
use crate::golden::MODEL;
use crate::model::{Device, DeviceHistory, JoinOrLeave, OutageSeenBy, Roam};
use crate::time;

const AT: i64 = 1_794_091_483;

fn model() -> HomeNetwork {
    serde_json::from_str(MODEL).unwrap()
}

fn count(history: &History, table: &str) -> i64 {
    history
        .db
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

/// Each move recorded, as `kind mac node band from_node from_band`.
fn moves(history: &History) -> Vec<String> {
    let mut statement = history
        .db
        .prepare(
            "SELECT kind, mac, node, band, from_node, from_band FROM moves
                 JOIN devices ON devices.id = moves.device ORDER BY at, mac",
        )
        .unwrap();
    statement
        .query_map([], |row| {
            let words: Vec<String> = (0..6)
                .map(|i| {
                    row.get::<_, Option<String>>(i)
                        .map(Option::unwrap_or_default)
                })
                .collect::<Result<_, _>>()?;
            Ok(words.join(" ").trim_end().to_owned())
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn a_network_read_samples_each_device_node_and_radio() {
    let mut history = History::in_memory();

    history.record(&model(), AT, true).unwrap();

    assert_eq!(count(&history, "device_samples"), 15);
    assert_eq!(count(&history, "node_samples"), 3);
    assert_eq!(count(&history, "radio_samples"), 9);
    let (signal, rx, down): (i64, i64, f64) = history
        .db
        .query_row(
            "SELECT signal_dbm, rx_kbps, down_mbps FROM device_samples
                 JOIN devices ON devices.id = device WHERE mac = '02:00:00:00:00:08'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!((signal, rx, down), (-57, 1_200_900, 11.452_299));
}

#[test]
fn a_read_that_isnt_the_networks_samples_nothing() {
    let mut history = History::in_memory();

    history.record(&model(), AT, false).unwrap();

    assert_eq!(count(&history, "device_samples"), 0);
    assert_eq!(
        count(&history, "speed_tests"),
        10,
        "the speed tests of the last 30 days are kept"
    );
}

#[test]
fn reads_closer_than_the_sampling_pace_add_no_sample() {
    let mut history = History::in_memory();

    history.record(&model(), AT, true).unwrap();
    history.record(&model(), AT + 5, true).unwrap();
    history.record(&model(), AT + SAMPLE_EVERY_S, true).unwrap();

    assert_eq!(count(&history, "device_samples"), 30);
}

#[test]
fn a_speed_test_is_kept_once_however_often_it_is_read() {
    let mut history = History::in_memory();

    history.record(&model(), AT, true).unwrap();
    history.record(&model(), AT + 30, true).unwrap();

    assert_eq!(count(&history, "speed_tests"), 10);
}

#[test]
fn coming_going_and_roaming_are_recorded_between_watched_reads() {
    let mut history = History::in_memory();
    let first = model();
    let mut second = first.clone();
    second.devices.retain(|d| d.mac != "02:00:00:00:00:03");
    let mut roamer = second.devices.remove(0);
    roamer.node = Some("2002".into());
    roamer.band = Some("2.4 GHz".into());
    let mut newcomer = roamer.clone();
    newcomer.mac = "02:00:00:00:00:40".into();
    second.devices.extend([roamer, newcomer]);

    history.record(&first, AT, true).unwrap();
    history.record(&second, AT + 30, true).unwrap();

    assert_eq!(
        moves(&history),
        [
            "left 02:00:00:00:00:03 2002 5 GHz",
            "roamed 02:00:00:00:00:0a 2002 2.4 GHz 2001 5 GHz",
            "joined 02:00:00:00:00:40 2002 2.4 GHz",
        ]
    );
}

#[test]
fn nobody_comes_or_goes_on_the_first_read_or_after_a_gap() {
    let mut history = History::in_memory();
    let mut fewer = model();
    fewer.devices.truncate(3);

    history.record(&model(), AT, true).unwrap();
    history
        .record(&fewer, AT + WATCHED_GAP_S + 1, true)
        .unwrap();

    assert_eq!(moves(&history), Vec::<String>::new());
}

#[test]
fn what_is_older_than_30_days_is_dropped() {
    let mut history = History::in_memory();
    history.record(&model(), AT, true).unwrap();

    history.record(&model(), AT + KEPT_S + 1, true).unwrap();

    assert_eq!(count(&history, "device_samples"), 15);
    assert_eq!(
        count(&history, "speed_tests"),
        0,
        "the contract's are older"
    );
}

#[test]
fn an_outage_eero_records_is_kept_once_from_when_it_was_seen() {
    let mut history = History::in_memory();
    let mut model = model();
    model.internet.outages = vec![serde_json::json!({"kind": "isp"})];

    history.record(&model, AT, false).unwrap();
    history.record(&model, AT + 60, false).unwrap();

    let seen: i64 = history
        .db
        .query_row("SELECT seen FROM eero_outages", [], |row| row.get(0))
        .unwrap();
    assert_eq!(seen, AT);
}

#[test]
fn the_model_carries_the_last_hours_samples_oldest_first() {
    let mut history = History::in_memory();
    let mut model = model();
    for (minutes, signal) in [(90, -80), (50, -70), (20, -60)] {
        model.devices[0].signal_dbm = Some(signal);
        history.record(&model, AT - minutes * 60, true).unwrap();
    }

    history.draw(&mut model, AT).unwrap();

    let series = &model.devices[0].history;
    assert_eq!(series.signal_dbm, [Some(-70), Some(-60)]);
    assert_eq!(
        series.at,
        [time::from_unix(AT - 3000), time::from_unix(AT - 1200)]
    );
    assert_eq!(series.rx_mbps, [Some(1200.9), Some(1200.9)]);
    assert_eq!(model.nodes[0].history.bars.len(), 2);
    assert_eq!(model.nodes[0].radios[2].history.busy_pct.len(), 2);
}

#[test]
fn a_device_with_no_samples_has_an_empty_series() {
    let history = History::in_memory();
    let mut model = model();

    history.draw(&mut model, AT).unwrap();

    assert_eq!(model.devices[0].history, DeviceHistory::default());
    assert_eq!(model.devices[0].roams, Vec::<Roam>::new());
    assert_eq!(model.joins_and_leaves, Vec::<JoinOrLeave>::new());
}

#[test]
fn a_device_shows_its_last_days_roams_newest_first_and_at_most_20() {
    let mut history = History::in_memory();
    let mut model = model();
    let start = AT - DAY_S - 30 * 60;
    for read in 0..60 {
        let band = if read % 2 == 0 { "5 GHz" } else { "2.4 GHz" };
        model.devices[0].band = Some(band.into());
        history.record(&model, start + read * 60, true).unwrap();
    }
    let now = start + 59 * 60;

    history.draw(&mut model, now).unwrap();

    let roams = &model.devices[0].roams;
    assert_eq!(roams.len(), ROAMS_SHOWN);
    assert_eq!(roams[0].at, time::from_unix(now));
    assert_eq!(roams[0].band.as_deref(), Some("2.4 GHz"));
    assert_eq!(roams[0].from_band.as_deref(), Some("5 GHz"));
    assert_eq!(roams[0].from_node, roams[0].node);
}

#[test]
fn a_speed_test_eero_has_dropped_is_still_shown_in_its_place() {
    let mut history = History::in_memory();
    let mut model = model();
    history.record(&model, AT, false).unwrap();
    let dropped = model.internet.speed_tests.remove(1);

    history.draw(&mut model, AT).unwrap();

    assert_eq!(model.internet.speed_tests.len(), 15);
    assert_eq!(model.internet.speed_tests[1], dropped);
}

#[test]
fn an_outage_eero_recorded_is_drawn_as_eero_wrote_it() {
    let mut history = History::in_memory();
    let mut model = model();
    model.internet.outages = vec![serde_json::json!({"kind": "isp"})];
    history.record(&model, AT, false).unwrap();
    model.internet.outages.clear();

    history.draw(&mut model, AT + 60).unwrap();

    let outage = &model.internet.history.outages[0];
    assert_eq!(outage.seen_by, OutageSeenBy::Eero);
    assert_eq!((&outage.start, &outage.end), (&None, &None));
    assert_eq!(outage.eero, Some(serde_json::json!({"kind": "isp"})));
}

/// A look from home, out or not, and with what times.
fn look(home: bool, reachable: bool) -> Reading {
    Reading {
        home,
        gateway_ms: home.then_some(1.9),
        internet_ms: reachable.then_some(12.4),
        dns_ms: (home && reachable).then_some(14.8),
        reachable,
    }
}

/// The probe's outages drawn, as `start..end`, in seconds after `AT`.
fn probe_outages(history: &History, now: i64) -> Vec<String> {
    let mut model = model();
    history.draw(&mut model, now).unwrap();
    let after = |at: &Option<String>| {
        at.as_deref()
            .map(|at| (time::to_unix(at).unwrap() - AT).to_string())
            .unwrap_or_default()
    };
    model
        .internet
        .history
        .outages
        .iter()
        .filter(|outage| outage.seen_by == OutageSeenBy::Probe)
        .map(|outage| format!("{}..{}", after(&outage.start), after(&outage.end)))
        .collect()
}

fn looks(history: &mut History, from: i64, every: i64, found: &[(bool, bool)]) {
    for (n, (home, reachable)) in (0..).zip(found) {
        history
            .record_probe(&look(*home, *reachable), from + n * every)
            .unwrap();
    }
}

#[test]
fn an_outage_the_probe_sees_runs_from_the_first_look_out_to_the_first_back() {
    let mut history = History::in_memory();
    let (up, down) = ((true, true), (true, false));

    looks(&mut history, AT, 10, &[up, down, down, down, up, up, down]);

    assert_eq!(probe_outages(&history, AT + 60), ["60..", "10..40"]);
}

#[test]
fn looks_away_from_home_record_no_outage_and_leave_one_open() {
    let mut history = History::in_memory();
    let away_out = (false, false);

    looks(&mut history, AT, 10, &[away_out, away_out, (true, false)]);
    looks(&mut history, AT + 30, 10, &[away_out, (true, true)]);

    assert_eq!(probe_outages(&history, AT + 40), ["20..40"]);
}

#[test]
fn after_nobody_looked_an_outage_ends_at_the_last_look_that_saw_it() {
    let mut history = History::in_memory();
    let down = (true, false);

    looks(&mut history, AT, 10, &[down, down]);
    looks(&mut history, AT + 3600, 10, &[down, (true, true)]);

    assert_eq!(probe_outages(&history, AT + 3610), ["3600..3610", "0..10"]);
}

#[test]
fn the_model_carries_the_last_look_and_the_last_hours_looks() {
    let mut history = History::in_memory();
    let mut model = model();
    looks(&mut history, AT - 4000, 1000, &[(true, true); 5]);

    history.draw(&mut model, AT).unwrap();

    let probe = model.internet.probe.unwrap();
    assert_eq!(probe.at, time::from_unix(AT));
    assert_eq!(
        (probe.gateway_ms, probe.internet_ms, probe.dns_ms),
        (Some(1.9), Some(12.4), Some(14.8))
    );
    assert!(
        probe.home && probe.reachable,
        "the last look saw the home network and reached the internet"
    );
    let looks = &model.internet.history.probes;
    assert_eq!(looks.at.len(), 4, "the last hour's");
    assert_eq!(looks.reachable, [true; 4]);
}

/// The model of a busy network after an hour read at the fast pace: 32
/// devices, each with every figure, and the golden model's three nodes.
/// It comes to about 336 kB, a third of it the series' times: written
/// every 30 seconds that's about 11 kB/s into the runtime folder, which
/// is memory, and a few milliseconds of the shell's parsing.
#[test]
fn a_busy_hour_keeps_the_model_small_enough_to_write_every_30_seconds() {
    let mut history = History::in_memory();
    let mut model = model();
    let pattern = model.devices[4].clone();
    model.devices = (0..32)
        .map(|n| Device {
            mac: format!("02:00:00:00:01:{n:02x}"),
            ..pattern.clone()
        })
        .collect();
    for read in 0..120 {
        history.record(&model, AT + read * 30, true).unwrap();
    }

    history.draw(&mut model, AT + 119 * 30).unwrap();

    let bytes = serde_json::to_string(&model).unwrap().len();
    assert_eq!(model.devices[0].history.at.len(), 120);
    assert!(bytes < 350_000, "the model takes {bytes} bytes");
}

#[test]
fn a_history_of_another_shape_is_set_aside() {
    let folder = std::env::temp_dir().join(format!("eero-history-{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("history.sqlite");
    let _ = fs::remove_file(&path);
    Connection::open(&path)
        .unwrap()
        .execute_batch("PRAGMA user_version = 99;")
        .unwrap();

    let history = History::open(&path).unwrap();

    assert_eq!(history.shape().unwrap(), SHAPE);
    assert!(
        folder.join("history.sqlite.shape-99.1").exists(),
        "the history of shape 99 was set aside under a free name"
    );
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn a_history_of_this_shape_from_before_the_notices_keeps_what_it_holds() {
    let folder = std::env::temp_dir().join(format!("eero-told-{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("history.sqlite");
    let _ = fs::remove_file(&path);
    Connection::open(&path)
        .unwrap()
        .execute_batch(&format!(
            "BEGIN; {TABLES} PRAGMA user_version = {SHAPE};
                 INSERT INTO speed_tests VALUES ({AT}, 525.0, 48.1); COMMIT;"
        ))
        .unwrap();

    let mut history = History::open(&path).unwrap();
    history
        .record_told(&(Kind::Joined, "02:00:00:00:00:20".into()), AT)
        .unwrap();

    assert_eq!(count(&history, "speed_tests"), 1);
    assert!(
        history.said(Kind::Joined, "02:00:00:00:00:20"),
        "the join told before the notices is still said"
    );
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn a_change_is_said_once_told_and_an_outage_stands_until_told_back() {
    let mut history = History::in_memory();
    let start = time::from_unix(AT);

    assert!(
        !history.said(Kind::InternetOut, &start),
        "an outage not yet told isn't said"
    );
    history
        .record_told(&(Kind::InternetOut, start.clone()), AT)
        .unwrap();
    let out = history.out_since();
    history
        .record_told(&(Kind::InternetBack, start.clone()), AT + 60)
        .unwrap();

    assert!(
        history.said(Kind::InternetOut, &start),
        "the outage told stays said once the internet is back"
    );
    assert_eq!(out, Some(start));
    assert_eq!(history.out_since(), None);
}

fn notice(kind: Kind, subjects: &[&str], summary: &str, body: &str) -> Notice {
    Notice {
        about: None,
        summary: summary.into(),
        body: body.into(),
        urgency: crate::notices::Urgency::Normal,
        tells: subjects.iter().map(|s| (kind, (*s).to_owned())).collect(),
    }
}

/// The told changes drawn at `now`, as `seconds-after-AT kind summary: body`.
fn told(history: &History, now: i64) -> Vec<String> {
    let mut model = model();
    history.draw(&mut model, now).unwrap();
    model
        .told
        .iter()
        .map(|told| {
            let after = time::to_unix(&told.at).unwrap() - AT;
            format!("{after} {} {}: {}", told.kind, told.summary, told.body)
        })
        .collect()
}

#[test]
fn the_model_carries_what_each_notice_said_newest_first_and_once() {
    let mut history = History::in_memory();
    let out = notice(
        Kind::InternetOut,
        &["t0"],
        "The internet is out",
        "Nothing.",
    );
    let updated = notice(
        Kind::Updated,
        &["2001 v7.18.0-1", "2002 v7.18.0-1"],
        "The nodes updated to v7.18.0-1",
        "They were on v7.17.4-5.",
    );
    let event = notice(Kind::Event, &["e"], "Guest network turned on.", "eero's.");

    history.record_notice(&out, AT).unwrap();
    history.record_notice(&updated, AT + 60).unwrap();
    history.record_notice(&event, AT + 90).unwrap();
    history
        .record_told(&(Kind::InternetBack, "t0".into()), AT + 120)
        .unwrap();
    history.record_notice(&out, AT + 180).unwrap();

    assert_eq!(
        told(&history, AT + 200),
        [
            "60 updated The nodes updated to v7.18.0-1: They were on v7.17.4-5.",
            "0 internet-out The internet is out: Nothing.",
        ],
        "one notice of two changes is one, a change settled without a notice and an \
         event eero lists aren't, and one told again keeps its first time"
    );
}

#[test]
fn the_model_carries_30_days_of_told_changes_and_at_most_50() {
    let mut history = History::in_memory();
    for n in 0..60 {
        let subject = n.to_string();
        let words = notice(
            Kind::Joined,
            &[subject.as_str()],
            &format!("device {n}"),
            "",
        );
        history.record_notice(&words, AT + n * 60).unwrap();
    }

    let shown = told(&history, AT + 59 * 60);
    let past = told(&history, AT + KEPT_S + 30 * 60);

    assert_eq!(shown.len(), 50);
    assert_eq!(shown[0], "3540 joined device 59: ");
    assert_eq!(past.len(), 29, "those past 30 days are left out");
}

#[test]
fn a_history_told_before_notices_kept_their_words_keeps_its_rows() {
    let folder = std::env::temp_dir().join(format!("eero-words-{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("history.sqlite");
    let _ = fs::remove_file(&path);
    Connection::open(&path)
        .unwrap()
        .execute_batch(&format!(
            "BEGIN; {TABLES} PRAGMA user_version = {SHAPE};
                 CREATE TABLE told (kind TEXT NOT NULL, subject TEXT NOT NULL,
                     at INTEGER NOT NULL, PRIMARY KEY (kind, subject)) WITHOUT ROWID;
                 INSERT INTO told VALUES ('joined', '02:00:00:00:00:20', {AT}); COMMIT;"
        ))
        .unwrap();

    let mut history = History::open(&path).unwrap();
    let joined = notice(Kind::Joined, &["02:00:00:00:00:21"], "phone 3", "New.");
    history.record_notice(&joined, AT + 60).unwrap();

    assert!(
        history.said(Kind::Joined, "02:00:00:00:00:20"),
        "the join told before notices kept their words is still said"
    );
    assert_eq!(told(&history, AT + 60), ["60 joined phone 3: New."]);
    drop(history);
    assert!(
        History::open(&path).is_ok(),
        "a history whose words were added opens again"
    );
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn what_was_told_is_dropped_after_30_days() {
    let mut history = History::in_memory();
    history
        .record_told(&(Kind::Updated, "2001 v7.18.0-1".into()), AT)
        .unwrap();

    history.record(&model(), AT + KEPT_S + 1, false).unwrap();

    assert!(
        !history.said(Kind::Updated, "2001 v7.18.0-1"),
        "an update told more than 30 days ago is no longer said"
    );
}
