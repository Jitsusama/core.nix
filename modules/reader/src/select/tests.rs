use super::*;

/// A line of the page at `baseline`, its characters 6 pt wide each.
#[expect(clippy::cast_precision_loss, reason = "a line's few characters")]
fn line(baseline: f64, text: &str) -> Line {
    let spans = (0..text.chars().count())
        .map(|i| (i as f64 * 6.0, (i + 1) as f64 * 6.0))
        .collect();
    Line {
        x_pt: 0.0,
        tail_x_pt: 0.0,
        baseline_pt: baseline,
        size_pt: 13.0,
        cap_pt: 9.1,
        text: text.to_string(),
        spans,
    }
}

/// The same, set in from the page's edge by `x_pt`, as a list's item
/// is from its bullet.
fn indented(baseline: f64, x_pt: f64, text: &str) -> Line {
    let mut set = line(baseline, text);
    set.x_pt = x_pt;
    set.tail_x_pt = x_pt;
    for span in &mut set.spans {
        *span = (span.0 + x_pt, span.1 + x_pt);
    }
    set
}

#[test]
fn a_list_is_marked_by_its_bullets_and_numbers_alone() {
    assert!(
        marker("\u{2022}"),
        "'\u{2022}' should read as a list's marker"
    );
    assert!(marker("4."), "'4.' should read as a list's marker");
    assert!(marker("b)"), "'b)' should read as a list's marker");
    assert!(
        !marker("4 items."),
        "'4 items.' should not read as a list's marker"
    );
    assert!(!marker(""), "nothing should not read as a list's marker");
    assert!(
        listed("- first item"),
        "'- first item' should read as a list's item"
    );
    assert!(
        listed("  12. twelfth"),
        "'  12. twelfth' should read as a list's item"
    );
    assert!(
        listed("> * quoted"),
        "'> * quoted' should read as a list's item"
    );
    assert!(
        !listed("| - | one |"),
        "'| - | one |' should not read as a list's item"
    );
    assert!(
        !listed("-5 degrees"),
        "'-5 degrees' should not read as a list's item"
    );
}

#[test]
fn a_lines_words_are_the_files_in_any_case() {
    assert!(
        holds_its_words("| Name | Size |", "NAME"),
        "'| Name | Size |' should hold the words NAME, in any case"
    );
    assert!(
        holds_its_words("A **bold** start", "A bold start,"),
        "'A **bold** start' should hold the words 'A bold start,'"
    );
    assert!(
        !holds_its_words("echo one", "SH"),
        "'echo one' should not hold the word SH"
    );
}

fn unit(top_pt: f64, first: usize, last: usize) -> Unit {
    Unit {
        top_pt,
        first,
        last,
    }
}

/// A page of a title, a paragraph of two lines, a list of two items
/// and a line of code, as the reader sets this file.
const FILE: &str = "# The Title\n\nSome **bold words** and a\n[link](https://example.com) here. Then more.\n\n- first item\n- second `code` item\n\n```\nlet x = 1;\n```\n";

fn page() -> Layout {
    let lines = vec![
        line(20.0, "The Title"),
        line(60.0, "Some bold words and a"),
        line(81.0, "link here. Then more."),
        line(120.0, "\u{2022}"),
        indented(120.0, 12.0, "first item"),
        line(141.0, "\u{2022}"),
        indented(141.0, 12.0, "second code item"),
        line(180.0, "let x = 1;"),
    ];
    let units = vec![
        unit(20.0, 1, 1),
        unit(60.0, 3, 4),
        unit(120.0, 6, 6),
        unit(141.0, 7, 7),
        unit(180.0, 10, 10),
    ];
    Layout::new(lines, units, &[5.0, 50.0, 110.0, 170.0], FILE.to_string())
}

fn at(line: usize, ch: usize) -> Pos {
    Pos { line, ch }
}

fn chars(page: &Layout, from: Pos, to: Pos) -> String {
    let mut selection = Selection::new(Kind::Chars, from);
    selection.cursor = to;
    page.copied(&selection)
}

#[test]
fn each_line_is_the_piece_marked_last_above_it() {
    let page = page();
    assert_eq!(
        page.of_line,
        [Some(0), Some(1), Some(1), Some(2), Some(3), Some(4)],
        "a bullet is the margin's, no line of its item's"
    );
    assert_eq!(
        page.paragraph,
        [0, 1, 2, 2, 3],
        "the list's items are one paragraph"
    );
}

#[test]
fn a_line_a_block_starts_with_that_no_piece_marks_is_no_pieces() {
    // An alert's kind, over its words, below the paragraph before it.
    let lines = vec![
        line(20.0, "Before."),
        line(60.0, "NOTE"),
        line(81.0, "Noted."),
    ];
    let units = vec![unit(20.0, 1, 1), unit(81.0, 4, 4)];
    let page = Layout::new(
        lines,
        units,
        &[5.0, 50.0],
        "Before.\n\n> [!NOTE]\n> Noted.\n".into(),
    );
    assert_eq!(page.of_line, [Some(0), None, Some(1)]);
}

#[test]
fn v_starts_on_the_first_word_wholly_in_view() {
    let page = page();
    assert_eq!(page.first_in_view(0.0, 400.0), Some(at(0, 0)));
    assert_eq!(
        page.first_in_view(10.0, 400.0),
        Some(at(1, 0)),
        "the title's top is cut"
    );
    assert_eq!(
        page.first_in_view(100.0, 400.0),
        Some(at(3, 0)),
        "a bullet is no word"
    );
}

#[test]
fn the_letters_and_arrows_move_by_characters_and_lines_keeping_across() {
    let page = page();
    let mut s = Selection::new(Kind::Chars, at(1, 12));
    page.moved(&mut s, Motion::Down, 1);
    assert_eq!(s.cursor, at(2, 12));
    page.moved(&mut s, Motion::Down, 1);
    assert_eq!(
        s.cursor,
        at(3, 9),
        "past the bullet, which is no line, to the item's end"
    );
    page.moved(&mut s, Motion::Down, 1);
    assert_eq!(s.cursor, at(4, 10), "back across to where it was");
    page.moved(&mut s, Motion::Left, 3);
    assert_eq!(s.cursor, at(4, 7));
    page.moved(&mut s, Motion::Right, 99);
    assert_eq!(s.cursor, at(4, 15), "l stops at the line's end");
    page.moved(&mut s, Motion::LineStart, 1);
    assert_eq!(
        s.cursor,
        at(4, 0),
        "the item's first letter, not its bullet"
    );
    page.moved(&mut s, Motion::LineEnd, 1);
    assert_eq!(s.cursor, at(4, 15));
    page.moved(&mut s, Motion::Up, 3);
    assert_eq!(s.cursor, at(1, 17));
}

#[test]
fn w_b_and_e_go_by_words_as_vi_does() {
    let page = page();
    let mut s = Selection::new(Kind::Chars, at(1, 0));
    page.moved(&mut s, Motion::WordOn, 1);
    assert_eq!(s.cursor, at(1, 5));
    page.moved(&mut s, Motion::WordEnd, 1);
    assert_eq!(s.cursor, at(1, 8));
    page.moved(&mut s, Motion::WordOn, 4);
    assert_eq!(s.cursor, at(2, 0), "a line's end is a blank between words");
    page.moved(&mut s, Motion::WordOn, 2);
    assert_eq!(s.cursor, at(2, 9), "punctuation is a word of its own");
    page.moved(&mut s, Motion::WordBack, 3);
    assert_eq!(s.cursor, at(1, 20));
}

#[test]
fn braces_go_by_paragraphs() {
    let page = page();
    let mut s = Selection::new(Kind::Chars, at(1, 4));
    page.moved(&mut s, Motion::ParagraphOn, 1);
    assert_eq!(s.cursor, at(3, 0));
    page.moved(&mut s, Motion::ParagraphOn, 1);
    assert_eq!(s.cursor, at(5, 0), "both items are one paragraph");
    page.moved(&mut s, Motion::ParagraphBack, 1);
    assert_eq!(s.cursor, at(3, 0));
    page.moved(&mut s, Motion::ParagraphBack, 2);
    assert_eq!(s.cursor, at(0, 0));
}

#[test]
fn a_word_and_a_sentence_are_selected_round_the_cursor() {
    let page = page();
    let mut s = Selection::new(Kind::Chars, at(1, 6));
    page.object(&mut s, Object::InWord);
    assert_eq!(s.ends(), (at(1, 5), at(1, 8)));
    page.object(&mut s, Object::AWord);
    assert_eq!(s.ends(), (at(1, 5), at(1, 9)));
    let mut s = Selection::new(Kind::Chars, at(1, 6));
    page.object(&mut s, Object::InSentence);
    assert_eq!(
        s.ends(),
        (at(1, 0), at(2, 9)),
        "a sentence goes across lines"
    );
    let mut s = Selection::new(Kind::Chars, at(2, 13));
    page.object(&mut s, Object::ASentence);
    assert_eq!(s.ends(), (at(2, 11), at(2, 20)));
}

#[test]
fn characters_copy_the_files_bytes_and_a_spans_markup_only_when_its_words_are_whole() {
    let page = page();
    assert_eq!(chars(&page, at(1, 5), at(1, 14)), "**bold words**");
    assert_eq!(
        chars(&page, at(1, 5), at(1, 8)),
        "bold",
        "half a span takes none of its markup"
    );
    assert_eq!(
        chars(&page, at(2, 0), at(2, 3)),
        "[link](https://example.com)"
    );
    assert_eq!(
        chars(&page, at(1, 16), at(2, 3)),
        "and a\n[link](https://example.com)",
        "across lines, the file's own break"
    );
    assert_eq!(chars(&page, at(4, 7), at(4, 10)), "`code`");
    assert_eq!(
        chars(&page, at(3, 0), at(4, 5)),
        "first item\n- second",
        "a bullet the file spells its own way"
    );
}

#[test]
fn i_backtick_selects_a_code_spans_words() {
    let page = page();
    let mut s = Selection::new(Kind::Chars, at(4, 8));
    page.object(&mut s, Object::InCode);
    assert_eq!(s.ends(), (at(4, 7), at(4, 10)));
    assert_eq!(page.copied(&s), "code", "inside, as Vim's i` takes it");
    let mut elsewhere = Selection::new(Kind::Chars, at(3, 2));
    page.object(&mut elsewhere, Object::InCode);
    assert_eq!(
        elsewhere.ends(),
        (at(3, 2), at(3, 2)),
        "outside code, nothing"
    );
}

#[test]
fn a_backtick_takes_the_ticks_and_the_blank_after_them() {
    let page = page();
    let mut s = Selection::new(Kind::Chars, at(4, 9));
    page.object(&mut s, Object::ACode);
    assert_eq!(s.ends(), (at(4, 7), at(4, 11)), "the page has no ticks");
    assert_eq!(page.copied(&s), "`code` ");
    s.put(at(4, 13));
    assert_eq!(
        page.copied(&s),
        "`code` it",
        "moved, it copies what it covers"
    );
    let at_end = Layout::new(
        vec![line(20.0, "Say code")],
        vec![unit(20.0, 1, 1)],
        &[5.0],
        "Say `code`\n".into(),
    );
    let mut s = Selection::new(Kind::Chars, at(0, 5));
    at_end.object(&mut s, Object::ACode);
    assert_eq!(s.ends(), (at(0, 3), at(0, 7)), "none after, so before");
    assert_eq!(at_end.copied(&s), " `code`");
}

#[test]
fn a_line_the_page_wraps_is_copied_and_shown_as_one() {
    let page = Layout::new(
        vec![
            line(20.0, "Some words wrap "),
            line(41.0, "here, and hy\u{ad}"),
            line(62.0, "phens too."),
            line(100.0, "Its own line."),
            line(121.0, "And another."),
        ],
        vec![unit(20.0, 1, 1), unit(100.0, 3, 4)],
        &[5.0, 90.0],
        "Some words wrap here, and hyphens too.\n\nIts own line.\nAnd another.\n".into(),
    );
    let mut s = Selection::new(Kind::Chars, at(0, 5));
    s.cursor = at(2, 4);
    assert_eq!(page.copied(&s), "words wrap here, and hyphens");
    assert_eq!(page.shown(&s), "words wrap here, and hyphens");
    s.cursor = at(4, 2);
    assert_eq!(
        page.copied(&s),
        "words wrap here, and hyphens too.\n\nIts own line.\nAnd",
        "the file's own breaks stay"
    );
    assert_eq!(
        page.shown(&s),
        "words wrap here, and hyphens too.\nIts own line.\nAnd"
    );
}

#[test]
fn pieces_copy_the_lines_they_were_written_on() {
    let page = page();
    let mut s = Selection::new(Kind::Pieces, at(2, 4));
    assert_eq!(
        page.copied(&s),
        "Some **bold words** and a\n[link](https://example.com) here. Then more.\n"
    );
    assert_eq!(
        page.pieces(&s).len(),
        2,
        "both of the paragraph's lines are drawn"
    );
    page.moved(&mut s, Motion::Down, 1);
    assert_eq!(
        page.copied(&s),
        "Some **bold words** and a\n[link](https://example.com) here. Then more.\n\n- first item\n"
    );
    let code = page.pieces_from(4, 1).expect("the code's line");
    assert_eq!(page.copied(&code), "let x = 1;\n");
}

#[test]
fn a_range_of_the_files_lines_is_taken_whole_and_kept_after_a_selection() {
    let page = page();
    let (whole, shown) = page.file_range(1, usize::MAX).expect("every line");
    assert!(shown, "the page shows some of the file's lines");
    assert_eq!(page.copied(&whole), FILE, "as :%y takes it");
    assert_eq!(
        page.pieces(&whole).len(),
        page.lines.len(),
        "all of it flashes"
    );
    let (list, _) = page.file_range(5, 7).expect("the list");
    assert_eq!(page.copied(&list), "\n- first item\n- second `code` item\n");
    assert_eq!(
        page.pieces(&list).len(),
        2,
        "the items flash, not the blank line"
    );
    let (blank, shown) = page.file_range(2, 2).expect("a blank line");
    assert!(!shown, "nothing to flash");
    assert_eq!(page.copied(&blank), "\n");
    assert!(
        page.file_range(0, 1).is_none(),
        "line 0 is before the first"
    );
    assert!(page.file_range(10, 12).is_none(), "past the end");
    assert!(page.file_range(11, 11).is_some(), "the last line");
    let mut s = Selection::new(Kind::Chars, at(1, 2));
    page.moved(&mut s, Motion::Down, 1);
    assert_eq!(page.file_lines_of(&s), Some((3, 4)));
    page.object(&mut s, Object::AParagraph);
    assert_eq!(page.file_lines_of(&s), Some((3, 5)), "its blank line too");
}

#[test]
fn g_lists_each_piece_by_its_first_matching_line_and_a_mark_by_its_line() {
    let page = page();
    let every = |query| page.matching(query, 1, usize::MAX);
    let more = every("more");
    assert_eq!(
        more,
        [(page.line_extent(2).0, "link here. Then more.".to_string())]
    );
    let items: Vec<String> = every("item").into_iter().map(|(_, w)| w).collect();
    assert_eq!(items, ["first item", "second code item"]);
    assert!(every("Item").is_empty(), "a capital matches its case");
    assert_eq!(page.matching("item", 7, 7).len(), 1, "within its range");
    assert_eq!(
        page.words_at(page.line_extent(1).0),
        Some("Some bold words and a")
    );
    assert_eq!(
        page.words_at(page.line_extent(1).0 + 1.0),
        Some("link here. Then more.")
    );
    assert_eq!(page.words_at(10_000.0), None);
}

#[test]
fn ip_takes_the_paragraph_and_ap_the_blank_lines_after_it() {
    let page = page();
    let mut s = Selection::new(Kind::Chars, at(4, 2));
    page.object(&mut s, Object::InParagraph);
    assert_eq!(page.copied(&s), "- first item\n- second `code` item\n");
    assert_eq!(
        page.pieces(&s).len(),
        2,
        "both items' words, their bullets the margin's and left bare"
    );
    page.object(&mut s, Object::AParagraph);
    assert_eq!(page.copied(&s), "- first item\n- second `code` item\n\n");
    let code = page
        .paragraph_of(4, Object::InParagraph)
        .expect("the code's paragraph");
    assert_eq!(
        page.copied(&code),
        "```\nlet x = 1;\n```\n",
        "its fences with it"
    );
    let list = page
        .paragraph_of(3, Object::AParagraph)
        .expect("the list's paragraph");
    assert_eq!(
        page.copied(&list),
        "- first item\n- second `code` item\n\n",
        "y a p takes the blank line after it"
    );
}

#[test]
fn y_y_takes_the_piece_the_cursors_line_is_in() {
    let page = page();
    assert_eq!(page.piece_at(7.0, 400.0), Some(0), "the title's line");
    assert_eq!(
        page.piece_at(68.0, 400.0),
        Some(1),
        "the paragraph's second line is the paragraph's"
    );
    assert_eq!(page.piece_at(128.0, 400.0), Some(3), "the second item");
    assert_eq!(
        page.piece_at(0.0, 400.0),
        Some(0),
        "over the first piece, as an eyebrow is, the first below"
    );
    let two = page.pieces_from(2, 2).expect("two items");
    assert_eq!(page.copied(&two), "- first item\n- second `code` item\n");
}

#[test]
fn y_brace_takes_the_pieces_to_the_paragraphs_end() {
    let page = page();
    let rest = page.to_paragraph_end(2, 1).expect("the list");
    assert_eq!(page.copied(&rest), "- first item\n- second `code` item\n");
    let from_second = page.to_paragraph_end(3, 1).expect("its second item");
    assert_eq!(page.copied(&from_second), "- second `code` item\n");
    let two = page.to_paragraph_end(1, 2).expect("two paragraphs");
    assert_eq!(
        page.copied(&two),
        "Some **bold words** and a\n[link](https://example.com) here. Then more.\n\n- first item\n- second `code` item\n"
    );
    let past = page.to_paragraph_end(4, 9).expect("the last paragraph");
    assert_eq!(
        page.copied(&past),
        "let x = 1;\n",
        "a count past the end stops there"
    );
}

#[test]
fn o_goes_to_the_other_end() {
    let mut s = Selection::new(Kind::Chars, at(1, 2));
    s.cursor = at(2, 4);
    s.swap();
    assert_eq!((s.anchor, s.cursor), (at(2, 4), at(1, 2)));
}

#[test]
fn words_the_file_doesnt_hold_copy_as_the_page_sets_them() {
    let lines = vec![line(10.0, "NOTES \u{b7} TODAY")];
    let page = Layout::new(lines, Vec::new(), &[], String::new());
    assert_eq!(chars(&page, at(0, 0), at(0, 4)), "NOTES");
}

/// The file set as the reader sets it, its words read for selecting.
fn set(name: &str, markdown: &str) -> Layout {
    let dir = std::env::temp_dir().join(format!("reader-select-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(&file, markdown).expect("the file");
    let (page, _) = crate::page::Setter::new(file.to_str().expect("a path"))
        .set()
        .expect("set");
    std::fs::remove_dir_all(&dir).expect("cleaned");
    Layout::read(&page.frame, markdown.to_string())
}

/// The page's line holding `words`, past the eyebrow, which names the
/// file's folder by the test's process and the time it was written,
/// so holds a 12 when either does.
fn holding(page: &Layout, words: &str) -> usize {
    page.lines
        .iter()
        .position(|line| !line.text.starts_with("READER-SELECT-") && line.text.contains(words))
        .unwrap_or_else(|| panic!("no line holds {words}"))
}

#[test]
fn a_set_pages_lines_are_the_pieces_they_were_written_in() {
    let markdown = "# A Title\n\nThe lede.\n\n> [!NOTE]\n> Noted **words** here.\n\n- One item\n- Two items\n\n```sh\necho one\necho two\n```\n\nThe last words.\n";
    let page = set("pieces", markdown);
    let piece =
        |words: &str| page.of_line[holding(&page, words)].map(|unit| page.units[unit].first);
    assert_eq!(piece("A Title"), Some(1));
    assert_eq!(piece("The lede."), Some(3));
    assert_eq!(piece("NOTE"), None, "an alert's kind is no piece's");
    assert_eq!(piece("Noted"), Some(6));
    assert_eq!(piece("One item"), Some(8));
    assert_eq!(piece("Two items"), Some(9));
    assert!(
        !page.lines.iter().any(|line| marker(&line.text)),
        "a list's bullets are the margin's, no line to select"
    );
    let starts =
        |page: &Layout, words: &str| page.lines[holding(page, words)].text.starts_with(words);
    assert!(starts(&page, "One item"), "nor the start of one");
    let numbered = set("numbers", "1. One\n2. Two\n10. Ten\n\n- [ ] A task\n");
    assert!(
        !numbered.lines.iter().any(|line| marker(&line.text)),
        "nor are its numbers or a task's box"
    );
    for words in ["One", "Two", "Ten", "A task"] {
        assert!(
            starts(&numbered, words),
            "{words}'s line is {:?}, not its words alone",
            numbered.lines[holding(&numbered, words)].text
        );
    }
    assert_eq!(piece("echo one"), Some(12));
    assert!(
        !page.lines.iter().any(|line| line.text.contains("SH")),
        "the code's language, beside its first line, is no line to select"
    );
    let table = set("cells", "| Name | Size |\n|---|---|\n| one | 12 |\n");
    let cell = |words: &str| table.of_line[holding(&table, words)].map(|u| table.units[u].first);
    assert_eq!(
        cell("NAME"),
        Some(1),
        "a head's cells beside each other are its own"
    );
    assert_eq!(cell("SIZE"), Some(1));
    assert_eq!(cell("12"), Some(3));
    assert_eq!(piece("echo two"), Some(13));
    assert_eq!(piece("The last"), Some(16));
    assert_eq!(
        page.first_in_view(0.0, 1000.0),
        Some(Pos {
            line: holding(&page, "A Title"),
            ch: 0
        }),
        "v starts on the title, not the eyebrow over it"
    );
    // Copying a line of words by its characters takes its markup whole.
    let noted = holding(&page, "Noted");
    let mut s = Selection::new(Kind::Chars, Pos { line: noted, ch: 0 });
    page.moved(&mut s, Motion::LineEnd, 1);
    assert_eq!(page.copied(&s), "Noted **words** here.");
    // The code's paragraph is its fences and lines.
    let code = page.of_line[holding(&page, "echo two")].expect("a piece");
    let whole = page
        .paragraph_of(code, Object::InParagraph)
        .expect("its paragraph");
    assert_eq!(page.copied(&whole), "```sh\necho one\necho two\n```\n");
    assert_eq!(page.pieces(&whole).len(), 2, "both lines of code show it");
}

#[test]
fn a_selection_grounds_each_lines_cell_and_joins_a_paragraphs_lines() {
    let words = "Words that run on past the measure. ".repeat(12);
    let markdown = format!("# A Title\n\n## A Section\n\n{words}\n\nThe next paragraph.\n");
    let page = set("cells", &markdown);
    let (title, first, next) = (
        holding(&page, "A Title"),
        holding(&page, "Words"),
        holding(&page, "The next"),
    );
    assert!(next > first + 1, "the paragraph wraps");
    let theme = crate::theme::get();
    let world = crate::page::new_world();
    let pitch = crate::page::pitch_pt();
    let cell = |selection: &Selection, family: &str| {
        let pieces = page.pieces(selection);
        let grounds = grounds(&pieces);
        for (piece, ground) in pieces.iter().zip(&grounds) {
            let (top, foot, from, to) = *ground;
            // The face the line is set in, whose capitals the run
            // carries: a title's weight stands shorter than its
            // family's regular, so the family's are only a check.
            let cap = piece.cap_pt;
            let regular = world.capitals_pt(family, piece.size_pt);
            assert!(
                (cap - regular).abs() < regular * 0.1,
                "its capitals, {cap} pt, are its face's, near {regular} pt"
            );
            let (over, under) = (piece.baseline_pt - cap - top, foot - piece.baseline_pt);
            assert!(
                (over - under).abs() < 0.05,
                "{ground:?} stands {over} pt over the capitals and {under} pt under the baseline at {}",
                piece.baseline_pt
            );
            assert_eq!(
                (from, to),
                (piece.from_pt, piece.to_pt),
                "its sides are its characters' edges"
            );
        }
        (pieces, grounds)
    };
    let mut paragraph = Selection::new(Kind::Chars, Pos { line: first, ch: 0 });
    paragraph.cursor = Pos { line: next, ch: 3 };
    let (pieces, grounds) = cell(&paragraph, &theme.levels.words.family);
    for (i, ground) in grounds.iter().enumerate() {
        assert!(
            (ground.1 - ground.0 - pitch).abs() < 0.01,
            "a line of words is grounded the page's pitch, {pitch} pt, tall, not {ground:?}"
        );
        if let Some(below) = grounds.get(i + 1) {
            if i + 2 < grounds.len() {
                assert!(
                    (below.0 - ground.1).abs() < 0.01,
                    "a paragraph's lines are one ground: {ground:?} then {below:?}"
                );
            } else {
                assert!(
                    below.0 > ground.1 + 1.0,
                    "two paragraphs' are apart: {ground:?} then {below:?}"
                );
            }
        }
    }
    assert_eq!(pieces.len(), next - first + 1);
    let titled = Selection::new(Kind::Chars, Pos { line: title, ch: 0 });
    let (pieces, grounds) = cell(&titled, &theme.levels.title.family);
    let tall = pitch.max(pieces[0].size_pt * 1.2);
    assert!(
        (grounds[0].1 - grounds[0].0 - tall).abs() < 0.01,
        "the title is grounded its size and a fifth, {tall} pt, tall, not {:?}",
        grounds[0]
    );
}

#[test]
fn a_search_match_is_found_by_where_it_starts() {
    let page = page();
    assert_eq!(page.place_of(60.0, 30.0), Some(at(1, 5)));
    assert_eq!(page.place_of(61.0, 30.0), None);
}
