use super::{
    Block, Mark, Opens, Place, Said, at, heading_of, keep_window_in, kept_at, letter, listed_in,
    mark, mark_at, moved, opening, pt_of, read_in, through, window_in, write_in,
};

#[test]
fn a_mark_kept_with_its_blocks_height_waits_on_the_first_screen_for_the_block_after() {
    // The first screen's page ends on block 4; the whole page has 5 after.
    let first = [(3, 100.0), (4, 140.0)];
    let whole = [(3, 100.0), (4, 140.0), (5, 300.0)];
    assert_eq!(
        mark_at(&first, false, 4, 20.0, Some(80.0)),
        None,
        "on the first screen, its block's height is unknown, so it waits"
    );
    assert_eq!(
        mark_at(&whole, true, 4, 20.0, Some(80.0)),
        Some(180.0),
        "on the whole page, as far through its block, now twice as tall"
    );
    assert_eq!(
        mark_at(&first, false, 3, 20.0, Some(80.0)),
        Some(110.0),
        "a block the first screen sets the next of lands at once"
    );
    assert_eq!(
        mark_at(&first, false, 4, 20.0, None),
        Some(160.0),
        "kept before heights were, it lands at its distance at once"
    );
    assert_eq!(
        mark_at(&whole, true, 9, 20.0, Some(80.0)),
        None,
        "a block the page has no longer"
    );
}

/// A block of a page, a heading's words given.
fn block(number: usize, heading: Option<&str>) -> Block {
    Block {
        number,
        top_pt: f64::from(u32::try_from(number).unwrap_or(0)) * 100.0,
        lines: None,
        heading: heading.map(|words| (2, words.to_string())),
    }
}

#[test]
fn a_place_sits_under_the_nearest_heading_at_or_before_it() {
    let blocks = [
        block(0, None),
        block(1, Some("Installing")),
        block(2, None),
        block(3, Some("Using It")),
    ];
    assert_eq!(heading_of(&blocks, 2).as_deref(), Some("Installing"));
    assert_eq!(
        heading_of(&blocks, 3).as_deref(),
        Some("Using It"),
        "a heading's own block is under it"
    );
    assert_eq!(
        heading_of(&blocks, 0),
        None,
        "above the first heading, none"
    );
    assert_eq!(heading_of(&blocks, 9), None, "a block the page lacks, none");
}

#[test]
fn a_mark_on_a_headings_line_is_kept_under_that_heading() {
    // A heading's block starts at its capitals, a few points below its
    // line's top, which is where the cursor's line and so the mark are.
    let blocks = [
        block(0, Some("A Page")),
        block(1, None),
        block(2, Some("Further On")),
    ];
    let kept = mark(&blocks, 'a', (196.0, 210.0), Some(1000.0));
    assert_eq!(kept.said.heading.as_deref(), Some("Further On"), "{kept:?}");
    assert_eq!(kept.block, 2, "{kept:?}");
    assert_eq!(
        pt_of(
            &[(0, 0.0), (1, 100.0), (2, 200.0)],
            kept.block,
            kept.into_pt,
            kept.block_pt
        ),
        Some(196.0),
        "it's kept where its line's top is"
    );
    let words = mark(&blocks, 'b', (120.0, 134.0), Some(1000.0));
    assert_eq!(
        (words.block, words.said.heading.as_deref()),
        (1, Some("A Page")),
        "a line of words under a heading is in its own block"
    );
    assert_eq!(
        words.block_pt,
        Some(100.0),
        "a mark keeps how tall its block stood"
    );
}

#[test]
fn a_mark_says_its_lines_share_of_the_whole_page() {
    // The view goes no further than 700 down a 1000-point page, so a
    // view's share of 950 would be all of it; a mark there is a point.
    let blocks = [block(0, None), block(9, None)];
    let kept = mark(&blocks, 'a', (950.0, 964.0), Some(1000.0));
    let share = kept
        .said
        .through
        .expect("a whole page's mark says how far down");
    assert!((share - 0.95).abs() < 1e-9, "{share} isn't 950 of 1000");
    assert_eq!(
        mark(&blocks, 'a', (950.0, 964.0), None).said.through,
        None,
        "a page not yet set whole can't say"
    );
}

#[test]
fn how_far_down_is_a_share_of_the_furthest_the_view_goes() {
    assert!(
        (through(400.0, 1000.0) - 0.4).abs() < 1e-9,
        "400 down of 1000 is two fifths"
    );
    assert!(
        (through(1000.0, 1000.0) - 1.0).abs() < 1e-9,
        "the end is the whole"
    );
    assert!(
        (through(1200.0, 1000.0) - 1.0).abs() < 1e-9,
        "never past the whole"
    );
    assert!(
        (through(0.0, 0.0) - 1.0).abs() < 1e-9,
        "a page within its window is read"
    );
}

#[test]
fn a_file_opens_at_the_mark_asked_for_or_where_it_was_left() {
    let place = Place {
        block: 9,
        into_pt: 4.0,
        block_pt: None,
        at_end: true,
        marks: vec![Mark {
            letter: 'a',
            block: 3,
            into_pt: 7.0,
            block_pt: Some(20.0),
            said: Said::default(),
        }],
        said: Said::default(),
        left: None,
    };
    assert_eq!(
        opening(Some(&place), Some('a')),
        (Some(opens(3, 7.0, Some(20.0), false)), None)
    );
    assert_eq!(
        opening(Some(&place), Some('b')),
        (Some(opens(9, 4.0, None, true)), Some('b')),
        "no mark b, so where it was left, saying so"
    );
    assert_eq!(
        opening(Some(&place), None),
        (Some(opens(9, 4.0, None, true)), None)
    );
    // Where each opens on a page whose block 3 now stands 40 pt tall,
    // twice what it was: the mark half again as far in, and the place
    // left at the end at the end, however the page has changed.
    let starts = [(3, 100.0), (4, 140.0), (9, 300.0)];
    assert_eq!(opens(3, 7.0, Some(20.0), false).at(&starts), Some(114.0));
    assert_eq!(opens(9, 4.0, None, true).at(&starts), Some(f64::INFINITY));
    assert_eq!(opening(None, Some('a')), (None, Some('a')), "nothing kept");
    assert_eq!(letter("a"), Some('a'));
    assert_eq!(letter("Q"), Some('Q'));
    assert_eq!(letter("ab"), None, "one letter alone");
    assert_eq!(letter("1"), None);
    assert_eq!(letter(""), None);
}

/// Where a file opens, written short.
fn opens(block: usize, into_pt: f64, block_pt: Option<f64>, at_end: bool) -> Opens {
    Opens {
        block,
        into_pt,
        block_pt,
        at_end,
    }
}

#[test]
fn a_place_kept_before_it_said_where_still_reads() {
    let dir = std::env::temp_dir().join(format!("reader-old-place-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(&file, "# A page").expect("the file");
    let state = dir.join("state");
    let kept = kept_at(&state, &file).expect("where it's kept");
    std::fs::create_dir_all(kept.parent().expect("its folder")).expect("made");
    let old = serde_json::json!({
        "file": file.to_string_lossy(),
        "block": 4,
        "into_pt": 2.5,
        "at_end": false,
        "marks": [{"letter": "a", "block": 1, "into_pt": 0.0}],
    });
    std::fs::write(&kept, old.to_string()).expect("kept");
    let place = read_in(&state, &file).expect("read");
    assert_eq!(
        (place.block, place.left, place.said, place.block_pt),
        (4, None, Said::default(), None)
    );
    assert_eq!(place.marks.len(), 1);
    assert_eq!(place.marks[0].said, Said::default());
    assert_eq!(place.marks[0].block_pt, None, "no height kept, none read");
    let listed = listed_in(&state, 9);
    assert_eq!(listed.len(), 1, "listed by its file's time");
    let line: serde_json::Value = serde_json::from_str(&listed[0]).expect("JSON");
    assert!(line["left"].as_u64().is_some_and(|left| left > 0), "{line}");
    assert!(
        line.get("heading").is_none() && line.get("through").is_none(),
        "{line}"
    );
    std::fs::remove_dir_all(&dir).expect("cleaned");
}

#[test]
fn the_places_are_listed_newest_left_first_without_a_file_gone() {
    let dir = std::env::temp_dir().join(format!("reader-listed-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let state = dir.join("state");
    let keep = |name: &str, left: u64, heading: Option<&str>| {
        let file = dir.join(name);
        std::fs::write(&file, "# A page").expect("the file");
        let said = Said {
            heading: heading.map(str::to_string),
            through: Some(0.4),
        };
        let place = Place {
            block: 2,
            into_pt: 0.0,
            block_pt: None,
            at_end: false,
            marks: vec![Mark {
                letter: 'b',
                block: 1,
                into_pt: 0.0,
                block_pt: None,
                said: said.clone(),
            }],
            said,
            left: Some(left),
        };
        write_in(&state, &file, &place).expect("kept");
        file
    };
    keep("older.md", 100, None);
    keep("newer.md", 300, Some("Installing"));
    let gone = keep("gone.md", 200, None);
    std::fs::remove_file(gone).expect("gone");
    let listed: Vec<serde_json::Value> = listed_in(&state, 9)
        .iter()
        .map(|line| serde_json::from_str(line).expect("JSON"))
        .collect();
    let files: Vec<&str> = listed
        .iter()
        .filter_map(|line| line["file"].as_str()?.rsplit('/').next())
        .collect();
    assert_eq!(files, ["newer.md", "older.md"]);
    assert_eq!(listed[0]["heading"], "Installing");
    assert_eq!(listed[0]["through"], 0.4);
    assert_eq!(listed[0]["left"], 300);
    assert_eq!(listed[0]["marks"][0]["letter"], "b");
    assert_eq!(listed[0]["marks"][0]["heading"], "Installing");
    assert!(listed[1].get("heading").is_none(), "no heading, none said");
    assert_eq!(listed_in(&state, 1).len(), 1, "no more than asked");
    std::fs::remove_dir_all(&dir).expect("cleaned");
}

#[test]
fn the_windows_width_is_kept_and_taken_back() {
    let state = std::env::temp_dir().join(format!("reader-window-{}", std::process::id()));
    assert_eq!(window_in(&state), None, "nothing kept yet");
    keep_window_in(&state, 360.0).expect("kept");
    assert_eq!(window_in(&state), Some(360.0));
    std::fs::remove_dir_all(&state).expect("cleaned");
}

#[test]
fn a_place_is_the_last_block_starting_above_it_and_the_way_into_it() {
    let starts = [(0, 36.0), (1, 120.0), (2, 400.0)];
    assert_eq!(at(&starts, 130.0), (1, 10.0));
    assert_eq!(at(&starts, 400.0), (2, 0.0));
    assert_eq!(
        at(&starts, 10.0),
        (0, 10.0),
        "above the first block is the page's top"
    );
    assert_eq!(pt_of(&starts, 1, 10.0, None), Some(130.0));
    assert_eq!(
        pt_of(&starts, 7, 0.0, None),
        None,
        "a block the page lacks has no place"
    );
}

#[test]
fn a_place_kept_at_another_width_lands_as_far_through_its_block() {
    // Block 1 stands 280 pt tall here.
    let starts = [(0, 36.0), (1, 120.0), (2, 400.0)];
    assert_eq!(
        pt_of(&starts, 1, 70.0, Some(280.0)),
        Some(190.0),
        "kept at this height, where it was"
    );
    assert_eq!(
        pt_of(&starts, 1, 70.0, Some(140.0)),
        Some(260.0),
        "half through a block half as tall, half through this one"
    );
    assert_eq!(
        pt_of(&starts, 2, 70.0, Some(140.0)),
        Some(470.0),
        "the last block's height is unknown, so the distance stands"
    );
    assert_eq!(
        pt_of(&starts, 1, 70.0, None),
        Some(190.0),
        "kept before heights were, the distance stands"
    );
    assert_eq!(
        pt_of(&starts, 1, -6.0, Some(140.0)),
        Some(114.0),
        "a heading's line over its capitals is its type's, and stands"
    );
    assert_eq!(
        pt_of(&starts, 1, 70.0, Some(1e-9)),
        Some(190.0),
        "a height no block stands is a file gone wrong, and the distance stands"
    );
    let after = [(0, 36.0), (1, 120.0), (2, 260.0)];
    assert_eq!(
        moved(&starts, &after, 330.0),
        Some(225.0),
        "three quarters through, set again at half the height"
    );
}

#[test]
fn a_place_kept_is_the_place_read_back() {
    let dir = std::env::temp_dir().join(format!("reader-place-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(&file, "# A page").expect("the file");
    let state = dir.join("state");
    let place = Place {
        block: 12,
        into_pt: 40.5,
        block_pt: Some(84.0),
        at_end: false,
        marks: vec![Mark {
            letter: 'a',
            block: 3,
            into_pt: 7.0,
            block_pt: Some(21.0),
            said: Said {
                heading: Some("Before".to_string()),
                through: Some(0.1),
            },
        }],
        said: Said {
            heading: Some("Installing".to_string()),
            through: Some(0.4),
        },
        left: Some(1_700_000_000),
    };
    write_in(&state, &file, &place).expect("kept");
    assert_eq!(read_in(&state, &file), Some(place));
    std::fs::remove_dir_all(&dir).expect("cleaned");
}
