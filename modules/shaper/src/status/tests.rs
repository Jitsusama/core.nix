//! What `shaper status` draws of a link, its tins and its spending in
//! each room, apart from the command so either can grow, and that
//! nothing from outside reaches the terminal as a control.

use super::*;

/// Four tins as CAKE counts them on a busy upload, with figures of
/// every width a day gives: a packet count in the millions beside
/// tins that carried none.
fn busy() -> Vec<qdisc::Tin> {
    let tin = |packets, drops, marks, average_us, peak_us| qdisc::Tin {
        packets,
        bytes: packets * 1200,
        backlog: 0,
        drops,
        marks,
        average_us,
        peak_us,
        flows: 0,
    };
    vec![
        tin(4_812_007, 1_204, 0, 12_400, 48_900),
        tin(912_334, 17, 3, 1_800, 9_100),
        tin(0, 0, 0, 0, 0),
        tin(88_210, 0, 0, 300, 2_200),
    ]
}

/// Where each of a line's cells ends, a cell being words set apart
/// from the next by two spaces or more.
fn ends(line: &str) -> Vec<usize> {
    let mut ends = Vec::new();
    let mut gap = 0;
    for (at, c) in line.char_indices() {
        if c == ' ' {
            gap += 1;
            if gap == 2 && at >= 2 {
                ends.push(at - 1);
            }
        } else {
            gap = 0;
        }
    }
    if gap < 2 && !line.trim().is_empty() {
        ends.push(line.trim_end().len());
    }
    ends
}

#[test]
fn every_figures_label_ends_where_its_figures_end() {
    for cells in [60, 80, 100, 200] {
        let lines = tins_table("sent", &busy(), &Pen::plain(), Room::Cells(cells));
        // The first column's words end where each word does; every
        // column after it is figures, flush right under its label.
        let head: Vec<usize> = ends(&lines[0]).into_iter().skip(1).collect();
        for line in &lines[1..] {
            let figures: Vec<usize> = ends(line).into_iter().skip(1).collect();
            assert_eq!(
                figures, head,
                "at {cells} columns, {line:?} doesn't end its figures under {:?}",
                lines[0]
            );
        }
    }
}

#[test]
fn every_figure_has_a_label_of_its_own() {
    let lines = tins_table("sent", &busy(), &Pen::plain(), Room::Cells(200));
    let labels = ends(&lines[0]).len();
    for line in &lines[1..] {
        assert_eq!(
            ends(line).len(),
            labels,
            "{line:?} holds a figure with no label over it, under {:?}",
            lines[0]
        );
    }
}

#[test]
fn a_unit_is_said_once_in_its_label() {
    let lines = tins_table("sent", &busy(), &Pen::plain(), Room::Cells(200));
    assert!(
        lines[0].contains("DELAY ms") && lines[0].contains("PEAK ms"),
        "{:?}",
        lines[0]
    );
    for line in &lines[1..] {
        assert!(!line.contains("ms"), "{line:?} says a unit its label says");
    }
}

#[test]
fn it_fits_the_room_from_sixty_columns_up() {
    for cells in [60, 80, 100, 200] {
        for line in tins_table("received", &busy(), &Pen::plain(), Room::Cells(cells)) {
            assert!(
                line.chars().count() <= cells,
                "{line:?} runs past {cells} columns"
            );
        }
    }
}

#[test]
fn at_eighty_columns_and_up_every_figure_stays() {
    for cells in [80, 100, 200] {
        let lines = tins_table("sent", &busy(), &Pen::plain(), Room::Cells(cells));
        assert!(
            lines[0].contains("MARKED") && lines[0].contains("PEAK ms"),
            "{:?}",
            lines[0]
        );
    }
}

fn shared(waiting_vpn: u32, waiting_direct: u32) -> lanes::Said {
    lanes::Said {
        joel: 1_740,
        vpn: 6_310,
        agents: 1_460,
        waiting_vpn,
        waiting_direct,
    }
}

#[test]
fn the_lanes_say_yours_first_then_the_vpn_s_then_agents_direct() {
    assert_eq!(
        lanes_said(&shared(12, 3)),
        [
            "the upload's lanes: yours 1.7, first; the VPN's 6.3; agents' direct 1.5 Mb/s",
            "agents waiting: 12 flows in the VPN, 3 direct",
        ],
        "the lanes weren't said as Joel reads them"
    );
}

#[test]
fn agents_waiting_on_one_lane_are_said_on_that_lane_alone() {
    let lines = lanes_said(&shared(0, 1));
    assert_eq!(
        lines.get(1).map(String::as_str),
        Some("agents waiting: 1 flow direct"),
        "{lines:?}"
    );
}

#[test]
fn no_agent_waiting_leaves_the_waiting_line_out() {
    assert_eq!(
        lanes_said(&shared(0, 0)).len(),
        1,
        "a line said that no agent waits"
    );
}

/// A link as it reads on a busy afternoon, its upload held for the
/// VPN's tunnel: a why of 120 cells, longer than the 103 that ran
/// off an 80-column terminal on 19 November.
fn held() -> LinkStatus {
    LinkStatus {
        name: "wlp0s20f3".to_owned(),
        ifb: "ifb4wlp0s20f3".to_owned(),
        network: "02:00:00:00:aa:00".to_owned(),
        up: 10_800,
        down: 2_000_000,
        state: "steady".to_owned(),
        baselines: vec![
            ("1.0.0.1".to_owned(), 17.0),
            ("1.1.1.1".to_owned(), 16.0),
            ("8.8.8.8".to_owned(), 16.0),
            ("9.9.9.9".to_owned(), 58.0),
        ],
        follows: Some("enp0s13f0u1u4".to_owned()),
        live: true,
        straight: false,
        heard: Some(crate::live::Heard {
            why: Some(
                "Upload 10.8 Mb/s: the VPN waited 89 ms inside its tunnel, so its own \
                 queue is held to 38% of its lane; held so for 36 s."
                    .to_owned(),
            ),
            plateau: Some(9_900.0),
            ..crate::live::Heard::default()
        }),
        lanes: Some(shared(12, 3)),
        room: Some(81_000),
    }
}

#[test]
fn a_download_held_for_a_call_says_so() {
    let lines = said(&held(), Room::Unbounded);
    assert!(
        lines.iter().any(|line| line
            == "  a call is on: the download is held to 81.0 Mb/s, so its queue stands here, \
                where the call goes first"),
        "a call's hold wasn't said: {lines:#?}"
    );
    let unturned = LinkStatus {
        straight: true,
        ..held()
    };
    let lines = said(&unturned, Room::Unbounded);
    assert!(
        lines.iter().any(|line| line
            == "  a call is on: the download would be held to 81.0 Mb/s, but it couldn't be \
                turned through its queue here, so it still queues at the router"),
        "a hold that couldn't turn the download was said to stand here: {lines:#?}"
    );
    let free = LinkStatus {
        room: None,
        ..held()
    };
    let lines = said(&free, Room::Unbounded);
    assert!(
        lines.iter().all(|line| !line.contains("a call is on")),
        "a hold was said with none: {lines:#?}"
    );
}

#[test]
fn a_link_s_lines_fit_the_room_from_sixty_columns_up() {
    for cells in [60, 80, 100, 200] {
        let text = said(&held(), Room::Cells(cells)).join("\n");
        assert_eq!(
            layout::overrun(&text, Room::Cells(cells)),
            None,
            "at {cells} columns a link's line runs past the edge"
        );
    }
}

#[test]
fn a_line_too_long_for_the_room_hangs_under_its_first_words() {
    let lines = said(&held(), Room::Cells(80));
    let at = lines
        .iter()
        .position(|line| line.starts_with("  Upload"))
        .expect("the why is said");
    let next = &lines[at + 1];
    assert!(
        next.starts_with("    ") && !next.starts_with("     "),
        "{next:?} doesn't hang two cells in from {:?}",
        lines[at]
    );
    // The first line keeps the cells only the lines under it hang by.
    // This why sets no item apart and leaves no widow, so with the
    // first line's whole room the next line's first word wouldn't
    // have fit on it; given the hung measure, as `hung` gave it, it
    // would have.
    let word = next
        .split_whitespace()
        .next()
        .expect("the next line has words");
    assert!(
        layout::width(&lines[at]) + 1 + layout::width(word) > 80,
        "{:?} left room for {word:?}, as it does when wrapped to the hung measure",
        lines[at]
    );
    let why = held().heard.and_then(|heard| heard.why).expect("a why");
    let hung: Vec<&str> = lines[at..]
        .iter()
        .take_while(|line| *line == &lines[at] || line.starts_with("    "))
        .flat_map(|line| line.split_whitespace())
        .collect();
    assert_eq!(
        hung,
        why.split_whitespace().collect::<Vec<_>>(),
        "the why lost or gained words in its wrap: {lines:#?}"
    );
}

#[test]
fn a_figure_stays_beside_its_unit() {
    // As the router's queue said it on 19 November, when at 70
    // columns its rate's figure ended one line and its unit began
    // the next: the shaper's own words, not a copy of them.
    let why = crate::why::say(
        crate::why::Why::Router { plateau: 10_300.0 },
        Some(9_800),
        None,
        4.0,
    );
    let mut link = held();
    if let Some(heard) = link.heard.as_mut() {
        heard.why = Some(why.clone());
    }
    for cells in 40..=80 {
        let lines = said(&link, Room::Cells(cells));
        // The why's own lines, so a figure said elsewhere can't stand
        // in for one the why parted.
        let at = lines
            .iter()
            .position(|line| line.starts_with("  Upload"))
            .expect("the why is said");
        let own: Vec<&String> = lines[at..]
            .iter()
            .take_while(|line| *line == &lines[at] || line.starts_with("    "))
            .collect();
        for figure in ["9.8 Mb/s", "10.3 Mb/s", "4 s."] {
            assert!(
                why.contains(figure) && own.iter().any(|line| line.contains(figure)),
                "at {cells} columns {figure:?} was parted: {own:#?}"
            );
        }
    }
}

#[test]
fn a_round_trip_s_host_stays_beside_its_time() {
    for cells in [50, 60, 70] {
        let lines = said(&held(), Room::Cells(cells));
        let at = lines
            .iter()
            .position(|line| line.starts_with("  quiet round trips"))
            .expect("the round trips are said");
        let own: Vec<&String> = lines[at..]
            .iter()
            .take_while(|line| line.starts_with("  quiet") || line.starts_with("    "))
            .collect();
        assert!(
            own.len() > 1,
            "at {cells} columns the round trips didn't wrap: {lines:#?}"
        );
        for (host, ms) in held().baselines {
            let trip = format!("{host} {ms:.0} ms");
            assert!(
                own.iter().any(|line| line.contains(&trip)),
                "at {cells} columns {trip:?} was parted: {lines:#?}"
            );
        }
    }
}

#[test]
fn into_a_pipe_each_of_a_link_s_lines_stays_whole() {
    let why = held().heard.and_then(|heard| heard.why).expect("a why");
    let lines = said(&held(), Room::Unbounded);
    assert!(
        lines.contains(&format!("{INSET}{why}")),
        "the why was parted into a pipe: {lines:#?}"
    );
    // The VM test greps these two, so each must stay on one line.
    for start in [
        "  quiet round trips: 1.0.0.1 17 ms",
        "  real-time traffic flows",
    ] {
        assert!(
            lines.iter().any(|line| line.starts_with(start)),
            "no line starts {start:?}: {lines:#?}"
        );
    }
    assert!(
        lines.iter().all(|line| !line.starts_with("    ")),
        "a line was hung into a pipe: {lines:#?}"
    );
    // The VM test looks for the link's name and the shaper's own
    // processor line too.
    assert!(
        lines[0].starts_with("wlp0s20f3  on "),
        "the heading was parted into a pipe: {lines:#?}"
    );
    let (first, later) = spent();
    let spent = first.said(&later, Room::Unbounded);
    assert!(
        spent
            .iter()
            .any(|line| line.starts_with("cpu: the shaper ") && line.ends_with("of a core")),
        "the shaper's processor line was parted into a pipe: {spent:#?}"
    );
}

#[test]
fn a_line_that_fits_the_room_stays_on_one_line() {
    let line = "  at enp0s13f0u1u4's rate: both reach the same router";
    let lines = said(&held(), Room::Cells(layout::width(line)));
    assert!(
        lines.iter().any(|each| each == line),
        "a line that fit its room was folded: {lines:#?}"
    );
}

#[test]
fn however_wide_the_terminal_a_link_s_lines_are_set_at_eighty() {
    let lines = said(&held(), Room::Cells(200));
    assert!(
        lines
            .iter()
            .all(|line| layout::width(line) <= layout::MEASURE),
        "a line ran past the measure at 200 columns: {lines:#?}"
    );
    assert!(
        lines.iter().any(|line| line.starts_with("    ")),
        "the why wasn't wrapped at the measure: {lines:#?}"
    );
}

#[test]
fn names_and_a_why_from_outside_draw_no_control_and_a_break_as_a_space() {
    // The leader's name fits its line and the why below wraps, so in
    // a terminal both of a hang's outcomes see the controls; piped, one.
    // A carriage return would send a terminal back over the line.
    let mut link = LinkStatus {
        name: "wlp0\u{1b}]0;owned\u{7}\rs20f3".to_owned(),
        network: "00:00\u{1b}]0;owned\u{7}:5e".to_owned(),
        follows: Some("enp0\u{1b}[2J\u{7}s13f0".to_owned()),
        ..held()
    };
    // A why wrapping at 60 and at the 80 that 200 is held to, so it hangs
    // in both, holding a break from outside between two words, apart
    // from the controls: drawn as a space, it starts no line and joins
    // neither word to the other. A terminal's wrap closes up every run of
    // blanks, so only the piped room sees a break drawn as two spaces or
    // kept: that room is the one pinning the space itself.
    if let Some(heard) = link.heard.as_mut() {
        heard.why = heard
            .why
            .take()
            .map(|why| format!("{why} \u{1b}[2J\u{7} held\ncleared"));
    }
    for room in [Room::Cells(60), Room::Cells(200), Room::Unbounded] {
        let lines = said(&link, room);
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("  at enp0") && line.contains("'s rate: both")),
            "the leader wasn't said into {room:?}: {lines:#?}"
        );
        // The heading's names are drawn whole on either side of their
        // controls, so a filter running past an escape's end, or a heading
        // left without them, fails here and not only where a control is.
        assert!(
            lines[0].starts_with("wlp0")
                && lines[0].contains("s20f3  on 00:00")
                && lines[0].ends_with(":5e"),
            "into {room:?} the heading didn't draw both names: {lines:#?}"
        );
        assert!(
            lines.iter().all(|line| !line.chars().any(char::is_control)),
            "into {room:?} a control or a line break was drawn: {lines:#?}"
        );
        // The why's own lines, so the round trips' hang can't stand in:
        // its last, hung under its first or piped as the one, ends on the
        // two words the break parted, with a space between them.
        let at = lines
            .iter()
            .position(|line| line.starts_with("  Upload"))
            .expect("the why is said");
        let piped = room == Room::Unbounded;
        let hung = lines[at + 1..]
            .iter()
            .take_while(|line| line.starts_with("    "))
            .last();
        let (last, form) = if piped {
            (Some(&lines[at]), "its one line")
        } else {
            (hung, "the last line hung under its first")
        };
        assert!(
            last.is_some_and(|line| line.ends_with(" held cleared")),
            "into {room:?} the why's {form} didn't end 'held cleared': {lines:#?}"
        );
    }
}

/// A second of the shaper's spending, as two readings of the
/// processor and its services.
fn spent() -> (Spent, Spent) {
    let at = Instant::now();
    let reading = |softirq: u64, total: u64, shaper: u64| Spent {
        at,
        cores: cpu::ticks(&format!(
            "cpu  {total} 0 0 0 0 0 {softirq} 0\ncpu0 {total} 0 0 0 0 0 {softirq} 0"
        )),
        services: vec![Some(shaper), Some(shaper / 10)],
    };
    let mut later = reading(30, 1_100, 62_000);
    later.at = at + Duration::from_secs(1);
    (reading(10, 100, 0), later)
}

#[test]
fn the_processor_s_lines_fit_the_room() {
    let (first, later) = spent();
    let whole = first.said(&later, Room::Unbounded);
    let words = |lines: &[String]| {
        lines
            .join(" ")
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    for cells in [40, 60] {
        let lines = first.said(&later, Room::Cells(cells));
        assert!(
            lines.iter().any(|line| line.starts_with("softirq")),
            "no softirq line: {lines:#?}"
        );
        assert!(
            lines.iter().all(|line| layout::width(line) <= cells),
            "at {cells} columns a processor line ran past the edge: {lines:#?}"
        );
        assert_eq!(
            words(&lines),
            words(&whole),
            "at {cells} columns a processor line lost words"
        );
        // Each line begins one as said whole or hangs two cells under
        // its first words.
        assert!(
            lines.iter().all(
                |line| whole.iter().any(|said| said.starts_with(line.as_str()))
                    || (line.starts_with("  ") && !line.starts_with("   "))
            ),
            "at {cells} columns a wrapped line didn't hang: {lines:#?}"
        );
    }
    assert!(
        first.said(&later, Room::Cells(40)).len() > whole.len(),
        "at 40 columns no processor line wrapped"
    );
}

#[test]
fn no_tins_read_is_said_rather_than_headed() {
    let lines = tins_table("sent", &[], &Pen::plain(), Room::Cells(80));
    assert_eq!(
        lines,
        ["  sent: tc showed no tins"],
        "a table with no rows was headed"
    );
}
