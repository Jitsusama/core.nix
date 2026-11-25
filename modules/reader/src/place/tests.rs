use super::{
    Block, Landed, Mark, Opens, Place, Said, Taking, as_read, at, heading_of, heights_known,
    keep_window_in, kept_at, landed_again, letter, line_in, listed_in, mark, mark_at, moved,
    on_lines, opening, pt_of, read_in, set_again, still_waiting, take_kept, through, window_in,
    write_in,
};
use std::collections::HashMap;

#[test]
fn marks_still_waiting_are_written_back_whole_only_from_the_first_screen() {
    let kept = |letter| Mark {
        letter,
        block: 9,
        into_pt: 20.0,
        block_pt: Some(80.0),
        width_pt: Some(400.0),
        said: Said {
            heading: Some("Far".into()),
            through: Some(0.9),
        },
    };
    let waiting = [kept('a'), kept('b')];
    assert_eq!(
        still_waiting(&waiting, false, |letter| letter == 'b'),
        vec![kept('a')],
        "on the first screen, as read, all it said kept, unless set again since"
    );
    assert_eq!(
        still_waiting(&waiting, true, |_| false),
        vec![],
        "on the whole page, a mark still waiting has lost its block, and goes"
    );
}

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
        mark_at(&first, true, 4, 20.0, Some(80.0)),
        Some(160.0),
        "with the rest failed, no block after comes, so it lands at its kept distance"
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
    let kept = mark(&blocks, 'a', (196.0, 210.0), Some(1000.0), 372.5);
    assert_eq!(kept.said.heading.as_deref(), Some("Further On"), "{kept:?}");
    assert_eq!(
        kept.width_pt,
        Some(372.5),
        "a mark keeps how wide the page it was measured on was"
    );
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
    let words = mark(&blocks, 'b', (120.0, 134.0), Some(1000.0), 372.5);
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
    let kept = mark(&blocks, 'a', (950.0, 964.0), Some(1000.0), 400.0);
    let share = kept
        .said
        .through
        .expect("a whole page's mark says how far down");
    assert!((share - 0.95).abs() < 1e-9, "{share} isn't 950 of 1000");
    assert_eq!(
        mark(&blocks, 'a', (950.0, 964.0), None, 400.0).said.through,
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
            width_pt: None,
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
        "marks": [
            {"letter": "a", "block": 1, "into_pt": 0.0},
            {"letter": "b", "block": 1, "into_pt": 0.0, "block_pt": 0.5, "width_pt": 0},
            {"letter": "c", "block": 1, "into_pt": 0.0, "block_pt": -3, "width_pt": -3},
            {"letter": "d", "block": 1, "into_pt": 0.0, "block_pt": "tall", "width_pt": "wide"},
        ],
    });
    std::fs::write(&kept, old.to_string()).expect("kept");
    let place = read_in(&state, &file).expect("read");
    assert_eq!(
        (place.block, place.left, place.said, place.block_pt),
        (4, None, Said::default(), None)
    );
    assert_eq!(place.marks.len(), 4);
    assert_eq!(place.marks[0].said, Said::default());
    assert_eq!(place.marks[0].block_pt, None, "no height kept, none read");
    for mark in &place.marks[1..] {
        assert_eq!(
            mark.block_pt, None,
            "a height no block stands is a file gone wrong, read as none: {mark:?}"
        );
        assert_eq!(
            mark.width_pt, None,
            "a width no page is set at is a file gone wrong, read as none: {mark:?}"
        );
    }
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
                width_pt: None,
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
        (0, -26.0),
        "above the first block, that far over it"
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
    assert_eq!(
        pt_of(&starts, 1, 900.0, Some(1.0)),
        Some(400.0),
        "a distance kept past its block's foot is a file gone wrong, held to the block"
    );
    let after = [(0, 36.0), (1, 120.0), (2, 260.0)];
    assert_eq!(
        moved(&starts, &after, (330.0, 330.0), &[]),
        Some(225.0),
        "three quarters through, set again at half the height"
    );
}

#[test]
fn a_line_moved_to_another_width_lands_on_a_line_of_its_own_block() {
    // Block 1 is twice as tall after; block 2 opens on a heading whose
    // line's top stands 4 pt over its capitals, where the block starts.
    let before = [(0, 36.0), (1, 120.0), (2, 400.0)];
    let after = [(0, 36.0), (1, 120.0), (2, 680.0)];
    assert_eq!(
        moved(&before, &after, (396.0, 410.0), &[]),
        Some(676.0),
        "a heading's line keeps its lift over the block, however the block before grows"
    );
    assert_eq!(
        moved(&before, &after, (30.0, 46.0), &[]),
        Some(30.0),
        "the page's first line, over its block's capitals, stays"
    );
    assert_eq!(
        moved(
            &before,
            &[(0, 50.0), (1, 134.0), (2, 414.0)],
            (10.0, 10.0),
            &[]
        ),
        Some(24.0),
        "a line over every block, as the eyebrow's, keeps as far above the first"
    );
    // Block 1's lines, 20 pt apart, with the line before it and the
    // heading after it, each a top and a baseline 14 pt below it.
    let lines =
        |tops: &[f64]| -> Vec<(f64, f64)> { tops.iter().map(|top| (*top, top + 14.0)).collect() };
    let five = lines(&[100.0, 124.0, 144.0, 164.0, 184.0, 204.0, 396.0]);
    let ten = lines(&[
        100.0, 124.0, 144.0, 164.0, 184.0, 204.0, 224.0, 244.0, 264.0, 284.0, 676.0,
    ]);
    let there = moved(&before, &after, (164.0, 178.0), &ten);
    assert_eq!(there, Some(204.0), "as far through, on the nearest line");
    assert_eq!(
        moved(&after, &before, (204.0, 218.0), &five),
        Some(164.0),
        "and back, on the line it left, never one higher each time"
    );
    let shorter = [(0, 36.0), (1, 120.0), (2, 260.0)];
    assert_eq!(
        moved(
            &before,
            &shorter,
            (124.0, 138.0),
            &lines(&[100.0, 124.0, 144.0, 256.0])
        ),
        Some(124.0),
        "a block's first line, its inset shrunk, stays in its block, not the line before"
    );
    assert_eq!(
        line_in(&before, 1, 390.0, &five),
        Some(204.0),
        "past its last line, still its own, never the next heading"
    );
    assert_eq!(
        line_in(&before, 1, 121.0, &lines(&[100.0, 146.0])),
        Some(146.0),
        "a first line set well inside its block, as a code block's inset sets it, \
         takes a mark landed over it, never the nearer line before"
    );
    assert_eq!(line_in(&before, 1, 300.0, &[]), None, "no lines, none");
}

#[test]
fn a_kept_mark_goes_on_a_line_of_its_block_and_is_kept_as_read_until_he_moves_it() {
    let kept = |letter, block, into_pt| Mark {
        letter,
        block,
        into_pt,
        block_pt: Some(80.0),
        width_pt: Some(400.0),
        said: Said::default(),
    };
    let landed_at = |letter, block, into_pt, pt| Landed {
        kept: kept(letter, block, into_pt),
        pt,
        on_line: false,
    };
    // Block 1's lines, a top and a baseline 14 pt below it; block 3 has
    // none read yet.
    let starts = [(0, 36.0), (1, 120.0), (2, 200.0), (3, 400.0)];
    let lines: Vec<(f64, f64)> = [100.0, 124.0, 144.0, 164.0, 184.0]
        .iter()
        .map(|top| (*top, top + 14.0))
        .collect();
    let mut landed = vec![
        landed_at('a', 1, 40.0, 160.0),
        landed_at('b', 1, 40.0, 160.0),
        landed_at('c', 3, 20.0, 420.0),
    ];
    let mut marks = HashMap::from([('a', 160.0), ('b', 300.0), ('c', 420.0)]);
    assert_eq!(
        on_lines(&mut landed, &marks, &starts, &lines),
        vec![('a', 164.0)],
        "a goes on its block's nearest line; b, moved since, is his; c waits for its lines"
    );
    let letters: Vec<char> = landed.iter().map(|one| one.kept.letter).collect();
    assert_eq!(letters, ['a', 'c'], "b is no longer a kept mark");
    marks.insert('a', 164.0);
    assert_eq!(
        as_read(&landed, 'a', 164.0),
        Some(&kept('a', 1, 40.0)),
        "on its line, written back as read, its block's height and all"
    );
    assert_eq!(
        as_read(&landed, 'c', 420.0),
        Some(&kept('c', 3, 20.0)),
        "waiting for its line, as read, never found again from its point alone"
    );
    assert_eq!(as_read(&landed, 'a', 184.0), None, "moved, it's his");
    // Set again at another width, block 1 twice as tall and block 3 last.
    let after = [(0, 36.0), (1, 120.0), (2, 280.0), (3, 480.0)];
    assert_eq!(
        landed_again(&mut landed, &marks, &after),
        vec![('a', 200.0), ('c', 500.0)],
        "each lands again from what was kept: a as far through, c as far in"
    );
    assert!(
        landed.iter().all(|one| !one.on_line),
        "each goes on a line of the new page once its lines are read"
    );
    marks.extend([('a', 200.0), ('c', 430.0)]);
    assert_eq!(
        landed_again(&mut landed, &marks, &starts),
        vec![('a', 160.0)],
        "c, moved since, is his, and moves with its own line"
    );
}

#[test]
fn kept_marks_are_taken_as_read_and_one_kept_without_a_height_takes_its_blocks() {
    // Each was measured on a page 400 points wide, but f, kept before a
    // mark said how wide, never learns a height.
    let kept = |letter, block, into_pt, block_pt| Mark {
        letter,
        block,
        into_pt,
        block_pt,
        width_pt: Some(400.0),
        said: Said::default(),
    };
    // The first screen's page, ending on block 2, whose height isn't known.
    let starts = [(0, 36.0), (1, 120.0), (2, 200.0)];
    let mut waiting = vec![
        kept('a', 1, 40.0, Some(80.0)),
        kept('b', 1, 20.0, None),
        kept('c', 2, 10.0, None),
        kept('d', 2, 10.0, Some(50.0)),
        kept('e', 1, 0.0, Some(80.0)),
        Mark {
            width_pt: None,
            ..kept('f', 1, 30.0, None)
        },
    ];
    let mut marks = HashMap::from([('e', 999.0)]);
    let mut landed = Vec::new();
    let first = Taking {
        whole: false,
        width_pt: 400.0,
    };
    take_kept(&mut waiting, &mut marks, &mut landed, &starts, first);
    let letters: Vec<char> = waiting.iter().map(|one| one.letter).collect();
    assert_eq!(letters, ['d'], "d waits for its block's height");
    assert_eq!(
        marks,
        HashMap::from([
            ('a', 160.0),
            ('b', 140.0),
            ('c', 210.0),
            ('e', 999.0),
            ('f', 150.0)
        ]),
        "each lands as far through its block, or as far in; e, set while it waited, is his"
    );
    let heights: Vec<(char, Option<f64>)> = landed
        .iter()
        .map(|one| (one.kept.letter, one.kept.block_pt))
        .collect();
    assert_eq!(
        heights,
        [
            ('a', Some(80.0)),
            ('b', Some(80.0)),
            ('c', None),
            ('f', None)
        ],
        "b takes its block's height here; c's block's isn't known on the first screen, \
         and f can't say it was measured at this width"
    );
    // The whole page, at another width, says nothing of c's block.
    let wider = [(0, 36.0), (1, 120.0), (2, 260.0), (3, 300.0)];
    heights_known(&mut landed, &wider, 500.0);
    assert_eq!(
        landed.get(2).map(|one| one.kept.block_pt),
        Some(None),
        "c learns no height from a page set at another width"
    );
    // The whole page, at the same width, has the block after c's, and its
    // block 1 stands taller than a and b were kept against.
    let whole = [(0, 36.0), (1, 120.0), (2, 220.0), (3, 280.0)];
    heights_known(&mut landed, &whole, 400.0);
    let heights: Vec<(char, Option<f64>)> = landed
        .iter()
        .map(|one| (one.kept.letter, one.kept.block_pt))
        .collect();
    assert_eq!(
        heights,
        [
            ('a', Some(80.0)),
            ('b', Some(80.0)),
            ('c', Some(60.0)),
            ('f', None)
        ],
        "c takes its block's height once the whole page has it; a and b keep theirs, \
         and f still can't say its width"
    );
    // Set again at another width, blocks 1 and 2 twice as tall: b and c
    // scale with them.
    let after = [(0, 36.0), (1, 120.0), (2, 280.0), (3, 400.0)];
    assert_eq!(
        landed_again(&mut landed, &marks, &after),
        vec![('a', 200.0), ('b', 160.0), ('c', 300.0), ('f', 150.0)],
        "b and c land as far through their blocks as a does; f, as far in as it was kept"
    );
}

#[test]
fn a_page_at_another_width_lends_no_kept_mark_its_blocks_height() {
    // b, kept without its block's height, first finds its block on a page
    // set at another width than its distance was kept at.
    let mut waiting = vec![Mark {
        letter: 'b',
        block: 1,
        into_pt: 20.0,
        block_pt: None,
        width_pt: Some(400.0),
        said: Said::default(),
    }];
    let (mut marks, mut landed) = (HashMap::new(), Vec::new());
    let wider = Taking {
        whole: true,
        width_pt: 500.0,
    };
    take_kept(
        &mut waiting,
        &mut marks,
        &mut landed,
        &[(0, 36.0), (1, 120.0), (2, 280.0)],
        wider,
    );
    assert_eq!(
        landed.first().map(|one| one.kept.block_pt),
        Some(None),
        "b takes no height its distance was never kept against"
    );
    // Back at the width it was kept at, it lands as far in as it was kept.
    assert_eq!(
        landed_again(&mut landed, &marks, &[(0, 36.0), (1, 120.0), (2, 200.0)]),
        vec![('b', 140.0)],
        "b lands where it was kept, not scaled by another width's block"
    );
}

#[test]
fn a_page_set_again_lands_kept_marks_from_what_was_kept_unless_edited() {
    let kept = Mark {
        letter: 'a',
        block: 1,
        into_pt: 40.0,
        block_pt: Some(80.0),
        width_pt: Some(400.0),
        said: Said::default(),
    };
    let mut landed = vec![Landed {
        kept,
        pt: 164.0,
        on_line: true,
    }];
    let mut marks = HashMap::from([('a', 164.0), ('z', 300.0)]);
    let after = [(0, 36.0), (1, 120.0), (2, 280.0)];
    set_again(&mut landed, &mut marks, &after, false, |pt| pt + 1000.0);
    assert_eq!(
        marks,
        HashMap::from([('a', 200.0), ('z', 1300.0)]),
        "a lands again from what was kept, never from its line; z goes as moved"
    );
    assert!(
        landed.iter().all(|one| !one.on_line),
        "a goes on a line again"
    );
    set_again(&mut landed, &mut marks, &after, true, |pt| pt + 1000.0);
    assert!(landed.is_empty(), "an edit lets the kept marks go");
    assert_eq!(
        marks,
        HashMap::from([('a', 1200.0), ('z', 2300.0)]),
        "and a goes as moved, as z does, not from what was kept"
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
            width_pt: Some(372.5),
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
