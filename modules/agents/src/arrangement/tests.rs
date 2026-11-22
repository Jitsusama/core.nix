use super::*;

fn held(records: &Value) -> Held {
    let mut held = Held::default();
    held.took(&json!({ "records": records }));
    held
}

fn desk_parts() -> (Held, Held, HashMap<String, String>) {
    let windows = held(&json!([
        {"key": "window:7", "id": 7, "app_id": "kitty", "opened": "pi-session.7",
         "workspace": 1, "column": 2, "down": 1, "floating": false},
        {"key": "window:3", "id": 3, "app_id": "google-chrome", "title": "Charts - Chrome",
         "workspace": 2, "column": 1, "down": 1, "floating": false},
        {"key": "window:5", "id": 5, "app_id": "slack", "title": "Slack",
         "workspace": 2, "column": null, "down": null, "floating": true},
        {"key": "window:9", "id": 9, "app_id": "kitty", "opened": "pi-session.9",
         "workspace": 1, "column": 1, "down": 2, "floating": false}
    ]));
    let workspaces = held(&json!([
        {"key": "workspace:1", "id": 1, "idx": 3, "name": "LEAD", "output": "DP-4"},
        {"key": "workspace:2", "id": 2, "idx": 1, "output": "eDP-1"}
    ]));
    // Each as the windows daemon sends it: the row whole, under `row`.
    let screens = screens(&held(&json!([
        {"key": "row:displays:display:DP-4", "kind": "row", "row": {"held": {
         "output": "DP-4", "screen": "HP Inc. HP Z27k G3 CN41", "standing": "lit"}}},
        {"key": "row:displays:display:eDP-1", "kind": "row", "row": {"held": {
         "output": "eDP-1", "screen": "LG Display 0x07C6", "standing": "dark"}}},
        {"key": "row:displays:display:HDMI-1", "kind": "row", "row": {"held": {
         "output": "HDMI-1", "screen": "Dell U2720", "standing": "off"}}},
        {"key": "row:displays:answer:scale:DP-4", "kind": "row", "row": {"held": {
         "output": "DP-4", "part": "scale"}}},
        {"key": "row:displays:display:DP-9", "kind": "row", "row": {"held": {
         "output": "DP-9"}}},
        {"key": "row:displays:off", "kind": "row", "row": {"held": {}}}
    ])));
    (windows, workspaces, screens)
}

/// A pairing that couldn't tell `address`'s window from another's.
fn untold(address: &str) -> Value {
    let mut one = json!({"address": address, "window": null});
    one[UNTOLD] = json!(true);
    one
}

/// A pairing in the kitty whose socket is `unix:@kitty-<pid>`.
fn in_kitty(mut one: Value, pid: &str) -> Value {
    one["kitty"] = json!({"id": "1", "within": format!("unix:@kitty-{pid}")});
    one
}

fn session(address: &str, id: &str, standing: &str, under: &str) -> Value {
    json!({"key": format!("session:{address}"), "standing": standing, "here": true,
               "presence": {"addresses": [address], "session": id, "orchestratedBy": under}})
}

#[test]
fn only_screens_laid_out_are_named_by_what_they_are() {
    let (_, _, screens) = desk_parts();
    assert_eq!(
        screens,
        HashMap::from([
            ("DP-4".into(), "HP Inc. HP Z27k G3 CN41".into()),
            ("eDP-1".into(), "LG Display 0x07C6".into()),
        ]),
        "a screen that's off or says not what it is, and rows that aren't screens, are left out"
    );
}

#[test]
fn each_running_session_here_is_kept_where_its_window_sits() {
    let (windows, workspaces, screens) = desk_parts();
    let desk = Desk {
        windows: &windows,
        workspaces: &workspaces,
        screens: &screens,
    };
    let records = [
        session("LEAD", "s-lead", "running", ""),
        session("KELP", "s-kelp", "running", "lead"),
        session("GONE", "s-gone", "closed", ""),
        session("LOST", "s-lost", "running", ""),
    ];
    let paired = [
        json!({"address": "LEAD", "window": "window:9"}),
        json!({"address": "KELP", "window": "window:7"}),
        json!({"address": "GONE", "window": "window:3"}),
        json!({"address": "LOST", "window": null}),
    ];
    let kept = now(records.iter().map(Session), &paired, &desk);
    assert_eq!(
        Value::Object(kept),
        json!({
            "s-lead": {"screen": "HP Inc. HP Z27k G3 CN41", "output": "DP-4",
                       "workspace": 3, "workspaceName": "LEAD", "column": 1,
                       "tile": 2, "leads": true},
            "s-kelp": {"screen": "HP Inc. HP Z27k G3 CN41", "output": "DP-4",
                       "workspace": 3, "workspaceName": "LEAD", "column": 2,
                       "tile": 1, "leads": false, "under": "LEAD"}
        }),
        "one leading another says so, and one under another names it; each on a named \
             workspace keeps its name, since named ones come back in another order; a closed \
             one and one with no window aren't kept"
    );
}

#[test]
fn other_tiled_windows_are_kept_by_program_and_title_oldest_first() {
    let (windows, workspaces, screens) = desk_parts();
    let desk = Desk {
        windows: &windows,
        workspaces: &workspaces,
        screens: &screens,
    };
    assert_eq!(
        others(&desk),
        [json!({"app": "google-chrome", "title": "Charts - Chrome",
                    "screen": "LG Display 0x07C6", "output": "eDP-1",
                    "workspace": 1, "column": 1, "tile": 1})],
        "kitty's windows and a floating one aren't kept"
    );
}

#[test]
fn this_boots_places_join_those_kept_and_the_boot_before_stays() {
    let kept = Arrangement::of(&json!({"v": 1,
            "boots": {"this": {"s-ended": {"column": 4}}, "last": {"s-old": {"column": 1}},
                      "older": {"s-older": {"column": 2}}},
            "others": {"last": [{"app": "slack"}]}}));
    let now = Map::from_iter([("s-new".to_owned(), json!({"column": 3}))]);
    let after = kept.kept(
        "this",
        "last",
        now,
        vec![json!({"app": "chrome"})],
        &Windowless::default(),
        0,
    );
    assert_eq!(
        after.said(),
        json!({"v": 1,
                "boots": {"this": {"s-ended": {"column": 4}, "s-new": {"column": 3}},
                          "last": {"s-old": {"column": 1}}},
                "others": {"this": [{"app": "chrome"}], "last": [{"app": "slack"}]}}),
        "one ended keeps its place, the boot being restored stays, an older one goes"
    );
    assert_eq!(
        after.arranged("last"),
        HashSet::from(["s-old".to_owned()]),
        "the boot being restored kept a place of these"
    );
    assert_eq!(
        Arrangement::of(&json!({"v": 2, "boots": {}})),
        Arrangement::default(),
        "another version is kept as none"
    );
}

#[test]
fn a_session_running_here_with_no_window_is_told_apart() {
    let (windows, workspaces, screens) = desk_parts();
    let desk = Desk {
        windows: &windows,
        workspaces: &workspaces,
        screens: &screens,
    };
    let mut away = session("AWAY", "s-away", "running", "");
    away["here"] = json!(false);
    let records = [
        session("SHOWN", "s-shown", "running", ""),
        session("SHUT", "s-shut", "running", ""),
        session("LAPSED", "s-lapsed", "running", ""),
        session("PARKED", "s-parked", "parked", ""),
        session("ENDED", "s-ended", "closed", ""),
        session("UNTOLD", "s-untold", "running", ""),
        away,
    ];
    let paired = [
        in_kitty(json!({"address": "SHOWN", "window": "window:7"}), "5"),
        in_kitty(json!({"address": "SHUT", "window": null}), "5"),
        in_kitty(json!({"address": "LAPSED", "window": "window:44"}), "5"),
        untold("UNTOLD"),
    ];
    let seen = unwindowed(records.iter().map(Session), &paired, &desk);
    assert_eq!(
        seen.closed,
        HashSet::from(["s-shut".to_owned(), "s-lapsed".to_owned()]),
        "one with no window paired, or one niri no longer lists; never one shown, parked, \
             ended or elsewhere, nor one whose window can't be told from another's"
    );
    assert_eq!(
        seen.heard,
        [
            "s-shown", "s-shut", "s-lapsed", "s-parked", "s-ended", "s-untold", "s-away"
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<HashSet<_>>(),
        "every session the node said is heard, however it stands"
    );
}

#[test]
fn plain_pis_are_told_apart_by_the_kitty_drawing_them() {
    let (windows, workspaces, screens) = desk_parts();
    let desk = Desk {
        windows: &windows,
        workspaces: &workspaces,
        screens: &screens,
    };
    // NAMELESS is shown with no kitty said, which windows.rs never
    // pairs, so it lights no terminal for BARE, whose kitty is unknown.
    let records = [
        session("SHOWN", "s-shown", "running", ""),
        session("SHUT", "s-shut", "running", ""),
        session("OTHER", "s-other", "running", ""),
        session("NAMELESS", "s-nameless", "running", ""),
        session("BARE", "s-bare", "running", ""),
    ];
    let paired = [
        in_kitty(json!({"address": "SHOWN", "window": "window:7"}), "5"),
        in_kitty(json!({"address": "SHUT", "window": null}), "5"),
        in_kitty(json!({"address": "OTHER", "window": null}), "9"),
        json!({"address": "NAMELESS", "window": "window:9"}),
    ];
    let seen = unwindowed(records.iter().map(Session), &paired, &desk);
    assert_eq!(
        (seen.closed, seen.unsure),
        (
            HashSet::from(["s-shut".to_owned()]),
            HashSet::from(["s-other".to_owned(), "s-bare".to_owned()]),
        ),
        "a pi gone from a kitty still drawing another was closed by hand; one whose kitty \
             draws none, or that no kitty is known for, may have gone with it"
    );
}

#[test]
fn with_no_session_window_left_none_is_taken_as_closed() {
    let (windows, workspaces, screens) = desk_parts();
    let desk = Desk {
        windows: &windows,
        workspaces: &workspaces,
        screens: &screens,
    };
    let records = [
        session("SHUT", "s-shut", "running", ""),
        session("LAPSED", "s-lapsed", "running", ""),
    ];
    let paired = [
        in_kitty(json!({"address": "SHUT", "window": null}), "5"),
        in_kitty(json!({"address": "LAPSED", "window": "window:44"}), "5"),
    ];
    assert_eq!(
        unwindowed(records.iter().map(Session), &paired, &desk).closed,
        HashSet::new(),
        "every session's window gone at once is their kitty or niri gone, as a crash \
             takes them, never windows closed by hand"
    );
}

#[test]
fn a_terminal_drawing_none_of_its_sessions_is_gone_not_closed_by_hand() {
    let (windows, workspaces, screens) = desk_parts();
    let desk = Desk {
        windows: &windows,
        workspaces: &workspaces,
        screens: &screens,
    };
    let hosted = |address: &str, id: &str| {
        let mut one = session(address, id, "running", "");
        one["presence"]["hosted"] = json!(true);
        one
    };
    // Every viewer is a window of the sessions kitty; OWN is a pi in a
    // kitty of Joel's own, still shown as the sessions kitty crashes.
    let records = [
        hosted("REEF", "s-reef"),
        hosted("KELP", "s-kelp"),
        session("OWN", "s-own", "running", ""),
    ];
    let own = json!({"address": "OWN", "window": "window:3",
                         "kitty": {"id": "1", "within": "unix:@kitty-5"}});
    let crashed = unwindowed(
        records.iter().map(Session),
        std::slice::from_ref(&own),
        &desk,
    );
    assert_eq!(
        (crashed.closed, crashed.unsure),
        (
            HashSet::new(),
            HashSet::from(["s-reef".to_owned(), "s-kelp".to_owned()]),
        ),
        "every window one terminal drew gone at once is that terminal gone, though \
             another's still shows"
    );
    let kelp = json!({"address": "KELP", "window": "window:7"});
    let shut = unwindowed(records.iter().map(Session), &[own, kelp], &desk);
    assert_eq!(
        (shut.closed, shut.unsure),
        (HashSet::from(["s-reef".to_owned()]), HashSet::new()),
        "one viewer gone while another of that terminal's shows was closed by hand"
    );
}

#[test]
fn a_window_that_cant_be_placed_is_kept_as_one_with_no_place() {
    let (windows, workspaces, screens) = desk_parts();
    let desk = Desk {
        windows: &windows,
        workspaces: &workspaces,
        screens: &screens,
    };
    let records = [
        session("LEAD", "s-lead", "running", ""),
        session("UNTOLD", "s-untold", "running", ""),
        session("FLOAT", "s-float", "running", ""),
        session("LOST", "s-lost", "running", ""),
        session("ENDED", "s-ended", "closed", ""),
    ];
    let paired = [
        json!({"address": "LEAD", "window": "window:9"}),
        untold("UNTOLD"),
        json!({"address": "FLOAT", "window": "window:5"}),
        json!({"address": "LOST", "window": null}),
        untold("ENDED"),
    ];
    let kept = now(records.iter().map(Session), &paired, &desk);
    let unplaced = json!({"unplaced": true});
    assert_eq!(
        (
            kept.get("s-untold"),
            kept.get("s-float"),
            kept.get("s-lost"),
            kept.get("s-ended"),
        ),
        (Some(&unplaced), Some(&unplaced), None, None),
        "one whose window can't be told apart, or floats, has a window all the same, so \
             Reopen Them All brings it back in one; one with none, or ended, has no place"
    );
    assert!(kept["s-lead"]["column"] == 1, "one tiled is placed");
}

#[test]
fn one_with_no_place_now_keeps_the_place_it_had() {
    let kept = Arrangement::of(&json!({"v": 1,
            "boots": {"this": {"s-tiled": {"column": 4, "closed": 1_000}}}}));
    let now = Map::from_iter([
        ("s-tiled".to_owned(), json!({"unplaced": true})),
        ("s-new".to_owned(), json!({"unplaced": true})),
    ]);
    let after = kept.kept("this", "", now, Vec::new(), &Windowless::default(), 2_000);
    assert_eq!(
        after.boots["this"],
        json!({"s-tiled": {"column": 4}, "s-new": {"unplaced": true}}),
        "one seen in a window it can't be placed by keeps where it last sat, unmarked, \
             since its window is back; one never placed is kept as having a window"
    );
}

#[test]
fn a_mark_stands_while_its_terminal_draws_none() {
    let kept = Arrangement::of(&json!({"v": 1,
            "boots": {"this": {"s-shut": {"column": 4, "closed": 1_000},
                               "s-last": {"column": 5}}}}));
    let unsure = Windowless {
        unsure: HashSet::from(["s-shut".to_owned(), "s-last".to_owned()]),
        ..Windowless::default()
    };
    let grace = u64::try_from(CLOSED_FOR.as_millis()).expect("minutes fit");
    let first = kept.kept("this", "", Map::new(), Vec::new(), &unsure, 2_000);
    assert_eq!(
        first.boots["this"],
        json!({"s-shut": {"column": 4, "closed": 1_000},
                   "s-last": {"column": 5, "orphaned": true}}),
        "the last window closing leaves each place: a mark already set stands, and \
             one unmarked is taken as gone with its terminal"
    );
    let gone = first.kept("this", "", Map::new(), Vec::new(), &unsure, 1_000 + grace);
    assert_eq!(
        gone.boots["this"],
        json!({"s-last": {"column": 5, "orphaned": true}}),
        "one marked while others showed is let go when its time comes"
    );
}

#[test]
fn a_window_shown_again_leaves_the_rest_its_terminal_lost_unsure() {
    let kept = Arrangement::of(&json!({"v": 1,
            "boots": {"this": {"s-reef": {"column": 1}, "s-kelp": {"column": 2}}}}));
    let both = HashSet::from(["s-reef".to_owned(), "s-kelp".to_owned()]);
    let crashed = Windowless {
        unsure: both.clone(),
        heard: both.clone(),
        ..Windowless::default()
    };
    let kelp_back = Windowless {
        closed: HashSet::from(["s-reef".to_owned()]),
        heard: both.clone(),
        ..Windowless::default()
    };
    let kelp = || Map::from_iter([("s-kelp".to_owned(), json!({"column": 3}))]);
    let grace = u64::try_from(CLOSED_FOR.as_millis()).expect("minutes fit");
    let first = kept.kept("this", "", Map::new(), Vec::new(), &crashed, 1_000);
    let second = first.kept("this", "", kelp(), Vec::new(), &kelp_back, 2_000);
    assert_eq!(
        second.boots["this"]["s-reef"].get("closed"),
        None,
        "one whose window went with its whole terminal isn't taken as closed by hand when \
             another of that terminal's is opened again"
    );
    let later = second.kept("this", "", kelp(), Vec::new(), &kelp_back, 2_000 + grace);
    assert!(
        later.boots["this"].get("s-reef").is_some(),
        "so its place stays"
    );
    let reef_shown = Map::from_iter([
        ("s-reef".to_owned(), json!({"column": 1})),
        ("s-kelp".to_owned(), json!({"column": 3})),
    ]);
    let heard = Windowless {
        heard: both,
        ..Windowless::default()
    };
    let shown = later.kept("this", "", reef_shown, Vec::new(), &heard, 3_000);
    let shut = shown.kept("this", "", kelp(), Vec::new(), &kelp_back, 4_000);
    assert_eq!(
        shut.boots["this"]["s-reef"]["closed"],
        json!(4_000),
        "once shown again, closing it by hand marks it"
    );
}

#[test]
fn a_mark_stands_for_one_the_node_hasnt_said() {
    let kept = Arrangement::of(&json!({"v": 1,
            "boots": {"this": {"s-shut": {"column": 4, "closed": 1_000}}}}));
    let grace = u64::try_from(CLOSED_FOR.as_millis()).expect("minutes fit");
    let unheard = Windowless::default();
    assert_eq!(
        kept.kept("this", "", Map::new(), Vec::new(), &unheard, 2_000)
            .boots["this"],
        kept.boots["this"],
        "a keep before the node's sessions are heard leaves each mark as it was"
    );
    assert_eq!(
        kept.kept("this", "", Map::new(), Vec::new(), &unheard, 1_000 + grace)
            .boots["this"],
        json!({}),
        "and one comes due all the same"
    );
}

#[test]
fn one_with_no_place_now_leaves_the_restored_boots_place_to_stand() {
    let kept = Arrangement::of(&json!({"v": 1,
            "boots": {"this": {}, "last": {"s-float": {"column": 2}}}}));
    let now = Map::from_iter([
        ("s-float".to_owned(), json!({"unplaced": true})),
        ("s-new".to_owned(), json!({"unplaced": true})),
    ]);
    let after = kept.kept("this", "last", now, Vec::new(), &Windowless::default(), 0);
    assert_eq!(
        after.boots["this"],
        json!({"s-new": {"unplaced": true}}),
        "one the boot being restored placed keeps going back there, rather than home"
    );
}

#[test]
fn a_let_go_already_past_is_looked_at_no_sooner_than_still() {
    assert_eq!(
        again_in(10_000, 4_000),
        Duration::from_secs(6),
        "one still to come is looked at when it comes"
    );
    assert_eq!(
        again_in(4_000, 10_000),
        STILL,
        "one gone by waits as long as the windows' stillness, never waking the daemon at \
             once over and over"
    );
}

#[test]
fn a_window_closed_while_the_boot_runs_on_lets_its_place_go() {
    let kept = Arrangement::of(&json!({"v": 1,
            "boots": {"this": {"s-shut": {"column": 4}, "s-ended": {"column": 5}}}}));
    let shut = Windowless {
        closed: HashSet::from(["s-shut".to_owned()]),
        ..Windowless::default()
    };
    let at = 1_000_000;
    let grace = u64::try_from(CLOSED_FOR.as_millis()).expect("minutes fit");
    let first = kept.kept("this", "", Map::new(), Vec::new(), &shut, at);
    assert_eq!(
        first.boots["this"],
        json!({"s-shut": {"column": 4, "closed": at}, "s-ended": {"column": 5}}),
        "one whose window closed while its session runs on is marked so; one that ended \
             keeps its place, as a boot's end ends them"
    );
    assert_eq!(
        first.let_go_at("this"),
        Some(at + grace),
        "looked at again once a boot's end would have taken it"
    );
    let later = first.kept("this", "", Map::new(), Vec::new(), &shut, at + grace - 1);
    assert_eq!(
        later, first,
        "within that, its first close is kept, not each look"
    );
    let gone = later.kept("this", "", Map::new(), Vec::new(), &shut, at + grace);
    assert_eq!(
        gone.boots["this"],
        json!({"s-ended": {"column": 5}}),
        "past it, its place is let go, so Reopen Them All brings it back with no window"
    );
    assert_eq!(gone.let_go_at("this"), None, "and nothing waits");
    let back = first.kept(
        "this",
        "",
        Map::from_iter([("s-shut".to_owned(), json!({"column": 2}))]),
        Vec::new(),
        &Windowless::default(),
        at + 1,
    );
    assert_eq!(
        back.boots["this"]["s-shut"],
        json!({"column": 2}),
        "a window opened again is kept afresh, unmarked"
    );
    let parked = first.kept(
        "this",
        "",
        Map::new(),
        Vec::new(),
        &Windowless {
            heard: HashSet::from(["s-shut".to_owned()]),
            ..Windowless::default()
        },
        at + 1,
    );
    assert_eq!(
        parked.boots["this"]["s-shut"],
        json!({"column": 4}),
        "one no longer running here without a window, parked or ended, is kept unmarked, \
             as before it was marked"
    );
    assert_eq!(parked.let_go_at("this"), None, "so nothing waits on it");
}

#[test]
fn the_boot_before_is_the_one_restored_else_the_other_kept() {
    let kept = Arrangement::of(&json!({"v": 1, "boots": {"this": {}, "last": {}}}));
    assert_eq!(
        kept.before("this", ""),
        Some("last"),
        "none restoring: the other"
    );
    assert_eq!(
        kept.before("this", "this"),
        Some("last"),
        "this boot is never before"
    );
    assert_eq!(
        kept.before("this", "never"),
        Some("last"),
        "one not kept isn't it"
    );
    assert_eq!(
        Arrangement::default().before("this", ""),
        None,
        "none kept, none"
    );
}

#[test]
fn places_are_kept_once_the_windows_are_still() {
    let start = Instant::now();
    let mut still = Still::default();
    assert_eq!(still.wait(start), None, "nothing moved, nothing waits");
    still.moved(start);
    let half = Duration::from_millis(500);
    assert_eq!(
        still.wait(start + half),
        Some(Duration::from_millis(1500)),
        "the rest of the wait"
    );
    assert!(
        !still.due(start + Duration::from_secs(1)),
        "not still long enough"
    );
    still.moved(start + Duration::from_secs(1));
    assert!(!still.due(start + STILL), "a move starts the wait again");
    assert!(
        still.due(start + Duration::from_secs(3)),
        "still long enough"
    );
    assert_eq!(
        still.wait(start + Duration::from_secs(4)),
        None,
        "kept, nothing waits"
    );
    still.again(start + Duration::from_secs(60));
    assert_eq!(
        still.wait(start + Duration::from_secs(10)),
        Some(Duration::from_secs(50)),
        "a place to let go later is waited for too"
    );
    still.moved(start + Duration::from_secs(10));
    assert!(
        still.due(start + Duration::from_secs(12)),
        "a move is kept first"
    );
    assert_eq!(
        still.wait(start + Duration::from_secs(12)),
        None,
        "and keeping it asks again when the place goes"
    );
    still.again(start + Duration::from_secs(60));
    assert!(
        still.due(start + Duration::from_secs(60)),
        "once it's time, its place is kept again"
    );
    still.moved(start + Duration::from_secs(70));
    still.again(start + Duration::from_secs(71));
    assert!(
        !still.due(start + Duration::from_secs(71)),
        "a place to let go while the windows move waits until they're still"
    );
    assert!(
        still.due(start + Duration::from_secs(72)),
        "and is kept then"
    );
}

#[test]
fn what_the_shell_kept_is_read_from_its_newest_folder() {
    let shells = std::env::temp_dir().join(format!("agents-arrangement-{}", std::process::id()));
    let _ = fs::remove_dir_all(&shells);
    let older = shells.join("older");
    let newer = shells.join("newer");
    fs::create_dir_all(&older).expect("a shell's folder");
    fs::create_dir_all(&newer).expect("another's");
    fs::write(
        older.join("session-arrangement.json"),
        json!({"v": 1, "boots": {"a": {}}}).to_string(),
    )
    .expect("the older is written");
    std::thread::sleep(Duration::from_millis(20));
    fs::write(
        newer.join("session-arrangement.json"),
        json!({"v": 1, "boots": {"b": {}}}).to_string(),
    )
    .expect("the newer is written");
    let read = Arrangement::from_shell(&shells);
    assert_eq!(
        read.boots.keys().collect::<Vec<_>>(),
        ["b"],
        "the newest one's"
    );
    let kept = shells.join("agents").join("arrangement.json");
    read.write(&kept).expect("it's kept");
    assert_eq!(Arrangement::read(&kept), Some(read), "and read back whole");
    assert_eq!(
        Arrangement::from_shell(&shells.join("none")),
        Arrangement::default(),
        "no shell, nothing kept"
    );
    let _ = fs::remove_dir_all(&shells);
}
