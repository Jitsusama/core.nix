use super::*;

#[test]
fn the_key_lists_groups_share_two_columns_as_evenly_as_they_can() {
    assert_eq!(columns_of(&[3, 3]), [0, 1]);
    // A tall group stands alone beside the rest.
    assert_eq!(columns_of(&[2, 10, 2, 2, 2]), [0, 1, 0, 0, 0]);
    let heights: Vec<usize> = crate::keys::listing()
        .iter()
        .map(|(_, rows)| rows.len())
        .collect();
    let columns = columns_of(&heights);
    let side = |column: usize| -> usize {
        heights
            .iter()
            .zip(&columns)
            .filter(|(_, c)| **c == column)
            .map(|(rows, _)| rows + 2)
            .sum()
    };
    assert!(
        side(0).abs_diff(side(1)) <= 4,
        "the reader's own list is {} rows beside {}",
        side(0),
        side(1)
    );
}

#[test]
fn quoting_keeps_what_is_typed() {
    assert_eq!(quoted(r#"a "b" \c"#), r#""a \"b\" \\c""#);
}

/// The key list set for a window this many points wide: its width and
/// height in points.
fn key_list_set(width_pt: f64) -> (f64, f64) {
    let mut world = page::new_world();
    world.set_main(&key_list(width_pt));
    let document = typst::compile::<PagedDocument>(&world)
        .output
        .unwrap_or_else(|errors| {
            panic!(
                "{:?}",
                errors
                    .iter()
                    .map(|e| e.message.to_string())
                    .collect::<Vec<_>>()
            )
        });
    let size = document.pages()[0].frame.size();
    (size.x.to_pt(), size.y.to_pt())
}

#[test]
fn the_key_list_fits_inside_its_window_at_any_width() {
    // A whole screen at 1.5 is 1920 points; half and a third are what
    // a window beside another takes, and 400 is narrower than either.
    for window in [400.0, 640.0, 960.0, 1920.0] {
        let (width, _) = key_list_set(window);
        assert!(
            width <= window - 2.0 * inset_pt(),
            "the key list is {width:.0} pt wide in a window {window} pt wide"
        );
    }
}

#[test]
fn a_way_to_a_key_splits_into_its_keys_and_keeps_names_and_words_whole() {
    assert_eq!(keys_of("Space ?"), (vec!["Space", "?"], None));
    assert_eq!(keys_of("gg"), (vec!["gg"], None));
    assert_eq!(keys_of("Page Down"), (vec!["Page Down"], None));
    assert_eq!(keys_of("Space Page Up"), (vec!["Space", "Page Up"], None));
    assert_eq!(keys_of("m and a letter"), (vec!["m"], Some("and a letter")));
    assert_eq!(
        keys_of("Space m and a letter"),
        (vec!["Space", "m"], Some("and a letter"))
    );
}

/// The baselines runs stand on, each once, from the top down.
fn lines_of<'a>(runs: impl Iterator<Item = &'a [f64; 4]>) -> Vec<f64> {
    let mut lines: Vec<f64> = runs.map(|t| t[1]).collect();
    lines.sort_by(f64::total_cmp);
    lines.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    lines
}

#[test]
fn the_key_list_sets_its_lines_on_the_pitch_and_its_spaces_by_rung() {
    let space = &crate::theme::get().space;
    let pt = |px: f64| px * crate::theme::PT_A_PX;
    let (pitch, beat) = (pt(space.pitch), pt(space.beat));
    let rung = &space.rung;
    let mut world = page::new_world();
    world.set_main(&key_list(1920.0));
    let document = typst::compile::<PagedDocument>(&world)
        .output
        .unwrap_or_else(|errors| {
            panic!(
                "{:?}",
                errors
                    .iter()
                    .map(|e| e.message.to_string())
                    .collect::<Vec<_>>()
            )
        });
    let page = &document.pages()[0].frame;
    let (mut texts, mut rects) = (Vec::new(), Vec::new());
    drawn(page, (0.0, 0.0), &mut texts, &mut rects);
    let near = |a: f64, b: f64, by: f64| (a - b).abs() < by;
    assert!(
        !texts.iter().any(|t| t[2] < 0.1),
        "every run has ink: {texts:?}"
    );
    let left = texts.iter().map(|t| t[0]).fold(f64::INFINITY, f64::min);
    assert!(
        near(left, pt(rung.inset), 1.0),
        "its words start {left} pt in, not the inset rung's {}",
        pt(rung.inset)
    );
    // Lines by baseline, every one on the beat from the first, since
    // each column is a stack of its own whose groups stand a beat
    // apart, so one column's lines may fall a beat off the other's.
    let baselines = lines_of(texts.iter());
    for line in &baselines {
        let beats = (line - baselines[0]) / beat;
        assert!(
            near(beats, beats.round(), 0.001),
            "a line at {line} pt is off the beat from the first at {} pt",
            baselines[0]
        );
    }
    // Down the first column, its names and keys flush with the inset,
    // each line a pitch from the one before, or a pitch and a beat
    // where a group ends; the note under both columns may stand lower.
    let first = lines_of(texts.iter().filter(|t| near(t[0], left, 0.01)));
    for pair in first[..first.len() - 1].windows(2) {
        let step = pair[1] - pair[0];
        assert!(
            near(step, pitch, 0.01) || near(step, pitch + beat, 0.01),
            "lines at {} and {} pt stand {step} pt apart, not a pitch of {pitch} or a pitch and a beat",
            pair[0],
            pair[1]
        );
    }
    // The first line's baseline stands where a row of the page a pitch
    // tall centres a note's capitals, the last line's, from the inset.
    let last = baselines[baselines.len() - 1];
    let cap = texts
        .iter()
        .find(|t| near(t[1], last, 0.01))
        .expect("a last line")[3];
    assert!(
        near(
            baselines[0],
            pt(rung.inset) + f64::midpoint(pitch, cap),
            0.01
        ),
        "its first baseline is {} pt down",
        baselines[0]
    );
    // Along a line, one run from the next stands a chord's tie, a
    // key's apart, or the groups rung from a key to its words and from
    // one column to the next; no dot sets them apart.
    assert!(
        !texts.is_empty() && !key_list(1920.0).contains('\u{b7}'),
        "chords stand apart by space, not a dot"
    );
    let mut gaps = Vec::new();
    for &line in &baselines {
        let mut along: Vec<&[f64; 4]> = texts.iter().filter(|t| near(t[1], line, 0.01)).collect();
        along.sort_by(|a, b| a[0].total_cmp(&b[0]));
        // Runs that touch are one word's, set in two faces.
        for pair in along.windows(2) {
            let gap = pair[1][0] - (pair[0][0] + pair[0][2]);
            if gap > 0.5 {
                gaps.push(gap);
            }
        }
    }
    let groups = pt(rung.groups);
    for gap in &gaps {
        assert!(
            [pt(rung.tie), pt(rung.apart)]
                .iter()
                .any(|r| near(*gap, *r, 0.05))
                || *gap >= groups - 0.05,
            "two runs stand {gap} pt apart, no rung: {gaps:?}"
        );
    }
    for rung_pt in [pt(rung.tie), pt(rung.apart), groups] {
        assert!(
            gaps.iter().any(|gap| near(*gap, rung_pt, 0.05)),
            "no two runs stand {rung_pt} pt apart: {gaps:?}"
        );
    }
}

#[test]
fn the_key_list_takes_two_columns_where_they_fit_and_one_where_not() {
    let (wide, wide_tall) = key_list_set(1920.0);
    let (narrow, narrow_tall) = key_list_set(640.0);
    assert!(
        wide > narrow,
        "{wide:.0} pt wide on a screen, {narrow:.0} pt in a third"
    );
    assert!(
        narrow_tall > wide_tall * 1.5,
        "one column stands taller than two"
    );
}

#[test]
fn labels_set_one_page_each_in_the_themes_ink() {
    let mut world = page::new_world();
    world.set_main(&labels(&["a".into(), "sd".into()]));
    let document = typst::compile::<PagedDocument>(&world)
        .output
        .unwrap_or_else(|errors| {
            panic!(
                "{:?}",
                errors
                    .iter()
                    .map(|e| e.message.to_string())
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(document.pages().len(), 2);
}

#[test]
fn a_labels_letters_stand_on_its_links_baseline_within_a_pixel() {
    let dir = std::env::temp_dir().join(format!("reader-label-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(
        &file,
        "# A Page\n\nWords before [a link](other.md) and after it.\n\n`code` then [another](#a-page).\n",
    )
    .expect("the file");
    let (page, _) = page::Setter::new(file.to_str().expect("a path"))
        .set()
        .expect("set");
    std::fs::remove_dir_all(&dir).expect("cleaned");
    let found = crate::links::links(&page.frame);
    assert_eq!(found.len(), 2);
    // Each link's baseline is its words' own, as search reads them.
    let lines = crate::text::lines(crate::text::runs(&page.frame));
    for link in &found {
        assert!(
            lines
                .iter()
                .any(|line| (line.baseline_pt - link.baseline_pt).abs() < 0.01),
            "{link:?}"
        );
    }
    let mut world = page::new_world();
    world.set_main(&labels(&["x".into(), "sd".into()]));
    let document = typst::compile::<PagedDocument>(&world)
        .output
        .expect("the labels set");
    // Scales 1.5 and 2, at their own size and zoomed, and the view's
    // top anywhere between two pixels.
    for scale in [1.5, 2.0] {
        for zoom in [1.0, 1.1, 1.0 / 1.1, 1.21] {
            let px = scale / 0.75 * zoom;
            for offset in [0.0, 0.25, 0.5, 0.75, 13.37] {
                for (link, label) in found.iter().zip(document.pages()) {
                    let label = rendered(label, px);
                    assert!(label.baseline_px > 0.0 && label.baseline_px < f64::from(label.height));
                    let top =
                        f64::from(crate::links::label_top(link, label.baseline_px, px, offset));
                    let words = link.baseline_pt * px - offset;
                    assert!(
                        (top + label.baseline_px - words).abs() <= 1.0,
                        "at {px} px a point, {offset} down: the label's letters at {}, the link's at {words}",
                        top + label.baseline_px
                    );
                }
            }
        }
    }
}

/// What a set page draws, in points from its top left: each run of
/// text as its left, baseline, advance and capitals' height, and each
/// rectangle as its left, top, width and height.
fn drawn(
    frame: &typst::layout::Frame,
    (dx, dy): (f64, f64),
    texts: &mut Vec<[f64; 4]>,
    rects: &mut Vec<[f64; 4]>,
) {
    for (place, item) in frame.items() {
        let (x, y) = (dx + place.x.to_pt(), dy + place.y.to_pt());
        match item {
            typst::layout::FrameItem::Text(text) => texts.push([
                x,
                y,
                text.width().to_pt(),
                text.font.metrics().cap_height.at(text.size).to_pt(),
            ]),
            typst::layout::FrameItem::Shape(shape, _) => {
                if let typst::visualize::Geometry::Rect(size) = shape.geometry {
                    rects.push([x, y, size.x.to_pt(), size.y.to_pt()]);
                }
            }
            typst::layout::FrameItem::Group(group) => drawn(
                &group.frame,
                (
                    x + group.transform.tx.to_pt(),
                    y + group.transform.ty.to_pt(),
                ),
                texts,
                rects,
            ),
            _ => {}
        }
    }
}

#[test]
fn the_prompts_row_sets_its_words_and_its_count_on_one_baseline() {
    let ask = |mark: &'static str, typed: &str, cursor, count, note: &str| Ask {
        serial: 1,
        keys: false,
        labels: Vec::new(),
        outline: Vec::new(),
        menu: None,
        note: note.into(),
        mark,
        typed: typed.into(),
        cursor,
        count,
        width_pt: 1080.0,
        px_per_pt: 2.0,
        page: crate::page::Width::WHOLE,
    };
    for (what, asked) in [
        ("a search typed", ask("/", "stands", true, Some((1, 1)), "")),
        (
            "a search found",
            ask("/", "stands", false, Some((1, 1)), ""),
        ),
        ("the command line", ask(":", "b", true, None, "Tab choose")),
    ] {
        let mut world = page::new_world();
        world.set_main(&source(&asked));
        let document = typst::compile::<PagedDocument>(&world)
            .output
            .unwrap_or_else(|errors| {
                panic!(
                    "{:?}",
                    errors
                        .iter()
                        .map(|e| e.message.to_string())
                        .collect::<Vec<_>>()
                )
            });
        let (mut texts, mut rects) = (Vec::new(), Vec::new());
        drawn(
            &document.pages()[0].frame,
            (0.0, 0.0),
            &mut texts,
            &mut rects,
        );
        assert!(texts.len() >= 2, "{what}: {texts:?}");
        let baseline = texts[0][1];
        assert!(
            texts.iter().all(|[_, y, ..]| (y - baseline).abs() < 0.01),
            "{what}: its words and count stand on {:?} pt, not one baseline",
            texts.iter().map(|[_, y, ..]| *y).collect::<Vec<_>>()
        );
        // The largest level's capitals, the typed Figure's, are centred
        // in the row.
        let cap = texts.iter().map(|t| t[3]).fold(0.0, f64::max);
        assert!(
            (baseline - cap / 2.0 - ROW_PT / 2.0).abs() < 0.05,
            "{what}: its capitals' middle is {} pt down a row of {ROW_PT}",
            baseline - cap / 2.0
        );
    }
}

#[test]
fn the_prompt_says_when_nothing_matches_only_once_something_is_typed() {
    let ask = |typed: &str, count| Ask {
        serial: 1,
        keys: false,
        labels: Vec::new(),
        outline: Vec::new(),
        menu: None,
        note: String::new(),
        mark: "/",
        typed: typed.into(),
        cursor: true,
        count,
        width_pt: 720.0,
        px_per_pt: 2.0,
        page: crate::page::Width::WHOLE,
    };
    assert!(source(&ask("cat", Some((0, 0)))).contains("Nothing matches."));
    assert!(!source(&ask("", Some((0, 0)))).contains("Nothing matches."));
    assert!(source(&ask("cat", Some((2, 5)))).contains("2 of 5"));
}

/// Where each run of text stands on a set page, in points from its
/// top left.
fn texts_at(frame: &typst::layout::Frame, dx: f64, dy: f64, found: &mut Vec<(f64, f64)>) {
    for (place, item) in frame.items() {
        let (x, y) = (dx + place.x.to_pt(), dy + place.y.to_pt());
        match item {
            typst::layout::FrameItem::Text(_) => found.push((x, y)),
            typst::layout::FrameItem::Group(group) => texts_at(
                &group.frame,
                x + group.transform.tx.to_pt(),
                y + group.transform.ty.to_pt(),
                found,
            ),
            _ => {}
        }
    }
}

/// A list over the page, set at each of Joel's scales, against the
/// ladder: a hairline of whole device pixels the window across at its
/// top, a beat of room inside it and at its foot, its rows two beats
/// apart with their capitals centred, and the chosen row's ground a mark
/// round its first and last ink, above its capitals and below its
/// baseline.
fn keeps_the_ladder(what: &str, ask: &dyn Fn(f64) -> String, rows: usize, chosen: usize) {
    let space = crate::theme::get().space;
    let beat = space.beat * crate::theme::PT_A_PX;
    let row = 2.0 * beat;
    let mark = space.rung.mark * crate::theme::PT_A_PX;
    for scale in [2.0_f64, 1.5] {
        let px = scale / crate::theme::PT_A_PX;
        let mut world = page::new_world();
        world.set_main(&ask(px));
        let document = typst::compile::<PagedDocument>(&world)
            .output
            .unwrap_or_else(|errors| {
                panic!(
                    "{:?}",
                    errors
                        .iter()
                        .map(|e| e.message.to_string())
                        .collect::<Vec<_>>()
                )
            });
        let frame = &document.pages()[0].frame;
        let (mut texts, mut rects) = (Vec::new(), Vec::new());
        drawn(frame, (0.0, 0.0), &mut texts, &mut rects);
        let near = |a: f64, b: f64| (a - b).abs() < 0.05;
        let at = format!("{what} at {scale}");
        let hair = rects
            .iter()
            .find(|[_, top, ..]| near(*top, 0.0))
            .unwrap_or_else(|| panic!("{at}: no hairline at its top in {rects:?}"));
        assert!(
            near(hair[2], frame.size().x.to_pt()),
            "{at}: its hairline is {} pt of {}",
            hair[2],
            frame.size().x.to_pt()
        );
        assert!(
            near(hair[3] * px, scale.round()),
            "{at}: its hairline is {} device pixels, not a logical pixel snapped",
            hair[3] * px
        );
        let mut baselines: Vec<f64> = texts.iter().map(|[_, y, ..]| *y).collect();
        baselines.sort_by(f64::total_cmp);
        baselines.dedup_by(|a, b| near(*a, *b));
        assert_eq!(baselines.len(), rows, "{at}: {baselines:?}");
        for pair in baselines.windows(2) {
            assert!(
                near(pair[1] - pair[0], row),
                "{at}: rows {} pt apart, not two beats ({row} pt)",
                pair[1] - pair[0]
            );
        }
        let cap = texts[0][3];
        assert!(
            near(baselines[0], hair[3] + beat + row / 2.0 + cap / 2.0),
            "{at}: its first baseline is {} pt down, under {} of hairline and a beat of {beat}",
            baselines[0],
            hair[3]
        );
        #[expect(clippy::cast_precision_loss, reason = "a few rows")]
        let tall = hair[3] + 2.0 * beat + rows as f64 * row;
        assert!(
            near(frame.size().y.to_pt(), tall),
            "{at}: it stands {} pt, not {tall}",
            frame.size().y.to_pt()
        );
        let line: Vec<&[f64; 4]> = texts
            .iter()
            .filter(|[_, y, ..]| near(*y, baselines[chosen]))
            .collect();
        let first = line.iter().map(|t| t[0]).fold(f64::INFINITY, f64::min);
        let last = line
            .iter()
            .map(|t| t[0] + t[2])
            .fold(f64::NEG_INFINITY, f64::max);
        let ground = rects
            .iter()
            .find(|[_, top, ..]| !near(*top, 0.0))
            .unwrap_or_else(|| panic!("{at}: no ground for the chosen row in {rects:?}"));
        let round = [
            first - ground[0],
            ground[0] + ground[2] - last,
            baselines[chosen] - cap - ground[1],
            ground[1] + ground[3] - baselines[chosen],
        ];
        assert!(
            round.iter().all(|gap| (gap - mark).abs() < 0.5),
            "{at}: the chosen ground stands {round:?} pt round its ink (left, right, over the capitals, under the baseline), not a mark ({mark} pt)"
        );
    }
}

#[test]
fn the_menu_and_the_lists_keep_the_ladder() {
    let ask = |px_per_pt: f64, outline: Vec<(u8, String, bool)>| Ask {
        serial: 1,
        keys: false,
        labels: Vec::new(),
        outline,
        menu: None,
        note: String::new(),
        mark: "",
        typed: String::new(),
        cursor: false,
        count: None,
        width_pt: 1080.0,
        px_per_pt,
        page: crate::page::Width::WHOLE,
    };
    let menu = Menu {
        lines: vec![
            ("bn[ext]".into(), "Next File".into(), "]b".into(), false),
            (
                "bp[revious]".into(),
                "Previous File".into(),
                "[b".into(),
                true,
            ),
        ],
        all: vec!["bn[ext]".into(), "bp[revious]".into()],
    };
    keeps_the_ladder(
        "the menu",
        &|px| menu_source(&ask(px, Vec::new()), &menu),
        2,
        1,
    );
    let rows = vec![
        (1, "The Reader's Cursor".into(), false),
        (2, "What Moves It".into(), true),
        (2, "What Acts There".into(), false),
    ];
    keeps_the_ladder("gO's outline", &|px| outline(&ask(px, rows.clone())), 3, 1);
}

#[test]
fn a_chosen_row_too_long_for_the_outline_sets_its_words_whole_where_its_room_is() {
    let long = "A heading far too long for the outline's row, which goes on and on \
                past where the page's measure ends, so it reads on in place";
    let ask = Ask {
        serial: 1,
        keys: false,
        labels: Vec::new(),
        outline: vec![(1, "Short".into(), false), (2, long.into(), true)],
        menu: None,
        note: String::new(),
        mark: "",
        typed: String::new(),
        cursor: false,
        count: None,
        width_pt: 500.0,
        px_per_pt: 2.0,
        page: crate::page::Width::WHOLE,
    };
    let mut world = page::new_world();
    world.set_main(&outline(&ask));
    let document = typst::compile::<PagedDocument>(&world)
        .output
        .unwrap_or_else(|errors| {
            panic!(
                "{:?}",
                errors
                    .iter()
                    .map(|e| e.message.to_string())
                    .collect::<Vec<_>>()
            )
        });
    let pages = document.pages();
    assert_eq!(
        pages.len(),
        3,
        "the list, the chosen row's words whole, a figure"
    );
    let room = outline_room(&ask).expect("a chosen row's room");
    assert_eq!(room.said, long);
    assert!(
        pages[1].frame.size().x.to_pt() > room.width_pt,
        "its words run past it"
    );
    assert!((pages[1].frame.size().y.to_pt() - row_pt()).abs() < 0.01);
    let figure = pages[2].frame.size().x.to_pt();
    assert!(figure > 2.0 && figure < 12.0, "a figure is {figure} pt");
    let mut texts = Vec::new();
    texts_at(&pages[0].frame, 0.0, 0.0, &mut texts);
    let chosen = texts
        .iter()
        .filter(|(_, y)| *y > room.top_pt && *y < room.top_pt + row_pt())
        .map(|(x, _)| *x)
        .fold(f64::INFINITY, f64::min);
    assert!(
        (chosen - room.left_pt).abs() < 0.5,
        "its words start at {chosen} pt, its room at {} pt",
        room.left_pt
    );
    let mut whole = Vec::new();
    texts_at(&pages[1].frame, 0.0, 0.0, &mut whole);
    let row = texts
        .iter()
        .filter(|(_, y)| *y > room.top_pt)
        .map(|(_, y)| y - room.top_pt)
        .fold(f64::INFINITY, f64::min);
    assert!(
        (whole[0].1 - row).abs() < 0.5,
        "the whole words stand on the row's baseline"
    );
}

#[test]
fn only_a_chosen_row_has_a_room_to_read_on_in() {
    let mut ask = Ask {
        serial: 1,
        keys: false,
        labels: Vec::new(),
        outline: vec![(1, "One".into(), false)],
        menu: None,
        note: String::new(),
        mark: "",
        typed: String::new(),
        cursor: false,
        count: None,
        width_pt: 720.0,
        px_per_pt: 2.0,
        page: crate::page::Width::WHOLE,
    };
    assert_eq!(outline_room(&ask), None);
    ask.outline = Vec::new();
    assert_eq!(outline_room(&ask), None, "the menu's rows don't read on");
}

#[test]
fn the_menu_sets_each_line_and_the_row_says_how_to_choose() {
    let mut ask = Ask {
        serial: 1,
        keys: false,
        labels: Vec::new(),
        outline: Vec::new(),
        menu: None,
        note: "Tab choose".into(),
        mark: ":",
        typed: "b".into(),
        cursor: true,
        count: None,
        width_pt: 720.0,
        px_per_pt: 2.0,
        page: crate::page::Width::WHOLE,
    };
    assert!(source(&ask).contains("\"Tab choose\""));
    let menu = Menu {
        lines: vec![
            ("bn[ext]".into(), "Next File".into(), "]b".into(), true),
            (
                "bp[revious]".into(),
                "Previous File".into(),
                "[b".into(),
                false,
            ),
        ],
        all: vec!["bn[ext]".into()],
    };
    let set = menu_source(&ask, &menu);
    assert!(set.contains("\"bn[ext]\"") && set.contains("\"Previous File\""));
    assert_eq!(
        set.matches("ink.selection").count(),
        1,
        "the chosen line's ground"
    );
    assert!(
        set.contains("(\"bn[ext]\", )"),
        "every command, as an array"
    );
    ask.menu = Some(menu);
    assert_eq!(slot(&ask), 3, "set where the outline is");
}
