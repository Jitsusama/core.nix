use super::tables::is_figure;
use super::{Costs, Pieces, picture_kind, program};
use crate::world::PageWorld;

/// The Typst program a page called notes walks to, set for a whole
/// window, with nothing of it set before.
fn walked(world: &PageWorld, markdown: &str) -> String {
    program(
        markdown,
        "notes",
        None,
        &Pieces::new(),
        world,
        crate::page::Width::whole(),
        &mut Costs::default(),
    )
}

#[test]
fn without_mitex_an_equation_shows_as_written_and_the_page_still_sets() {
    let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
    let mut costs = Costs::default();
    let typst = program(
        "Before $x^2$ after.\n",
        "page",
        None,
        &Pieces::new(),
        &world,
        crate::page::Width::whole(),
        &mut costs,
    );
    assert!(typst.contains("#broken(\"x^2\")"), "{typst}");
    assert!(!typst.contains("#mx("), "{typst}");
}

#[test]
fn a_title_opening_the_page_has_the_eyebrow_over_it_and_its_lede_after() {
    let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
    let walk = |markdown: &str| walked(&world, markdown);
    let titled = walk("# Title\n\nThe lede.\n\nThe words.\n");
    assert!(
        titled.contains("#about(titled: true)[#\"notes\"]"),
        "{titled}"
    );
    assert!(titled.contains("] <title>"), "{titled}");
    assert!(
        titled.contains("#lede[#metadata(\"s 3-3\")#\"The lede.\"]"),
        "{titled}"
    );
    assert!(
        titled.contains("#opening[#metadata(\"s 5-5\")#\"The words.\""),
        "{titled}"
    );
    let untitled = walk("The words.\n\n# Later\n\nMore.\n");
    assert!(untitled.contains("#about(titled: false)"), "{untitled}");
    assert!(
        !untitled.contains("] <title>"),
        "a later title isn't the page's: {untitled}"
    );
    assert!(
        untitled.contains("#par[#metadata(\"s 5-5\")#\"More.\""),
        "only the page's title has a lede: {untitled}"
    );
}

#[test]
fn each_piece_selected_whole_is_marked_with_the_lines_it_was_written_on() {
    let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
    let typst = program(
        "- One item\n- Two\n  - Inside\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n```sh\necho one\necho two\n```\n\n    indented\n",
        "notes",
        None,
        &Pieces::new(),
        &world,
        crate::page::Width::whole(),
        &mut Costs::default(),
    );
    assert!(
        typst.contains("[#metadata(\"s 1-1\")#\"One item\""),
        "{typst}"
    );
    assert!(
        typst.contains("[#metadata(\"s 2-2\")#\"Two \"#list("),
        "an item is its own words, down to the list in it: {typst}"
    );
    assert!(
        typst.contains("[#metadata(\"s 3-3\")#\"Inside\""),
        "{typst}"
    );
    assert!(typst.contains("head: ([#metadata(\"s 5-5\")"), "{typst}");
    assert!(typst.contains("rows: (([#metadata(\"s 7-7\")"), "{typst}");
    assert!(
        typst.contains("#box[#metadata(\"s 10-10\")]#text("),
        "a fenced block's code starts after its fence: {typst}"
    );
    assert!(
        typst.contains("#box[#metadata(\"s 11-11\")]#text("),
        "{typst}"
    );
    assert!(
        typst.contains("#box[#metadata(\"s 14-14\")]#text("),
        "an indented block's starts on its own first line: {typst}"
    );
}

#[test]
fn a_task_list_hangs_its_boxes_and_holds_each_tasks_words() {
    let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
    let typst = program(
        "- [x] Done it\n- [ ] Still to do\n",
        "notes",
        None,
        &Pieces::new(),
        &world,
        crate::page::Width::whole(),
        &mut Costs::default(),
    );
    assert!(typst.contains("#list(marker: no-mark, "), "{typst}");
    assert!(typst.contains("#task(true)[#\"Done it\"]"), "{typst}");
    assert!(typst.contains("#task(false)[#\"Still to do\"]"), "{typst}");
}

#[test]
fn a_quote_and_an_alert_hold_only_their_words() {
    let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
    let typst = program(
        "> Quoted words.\n\n> [!NOTE]\n> Noted words.\n",
        "notes",
        None,
        &Pieces::new(),
        &world,
        crate::page::Width::whole(),
        &mut Costs::default(),
    );
    assert!(
        typst.contains("#quote(block: true)[#par[#metadata(\"s 1-1\")#\"Quoted words.\"]"),
        "{typst}"
    );
    assert!(
        typst.contains("#alert(\"note\")[#par[#metadata(\"s 4-4\")#\"Noted words.\"]"),
        "the alert's kind is its eyebrow, not a paragraph too: {typst}"
    );
    assert!(
        !typst.contains("#\" \""),
        "a blank line between blocks: {typst}"
    );
}

#[test]
fn a_picture_alone_is_a_figure_and_one_among_words_stays_in_its_line() {
    let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
    let walk = |markdown: &str| walked(&world, markdown);
    let alone = walk("![What it shows](missing.png)\n");
    assert!(
        alone.contains("#alone[#picture(\"What it shows\")][]"),
        "one that can't be shown says its words once, with no caption: {alone}"
    );
    let among = walk("Words ![What it shows](missing.png) and more.\n");
    assert!(!among.contains("#alone"), "{among}");
    assert!(among.contains("#opening["), "{among}");
}

#[test]
fn the_pages_first_paragraph_of_words_opens_it() {
    let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
    let walk = |markdown: &str| walked(&world, markdown);
    // Past the title, its lede and a section's heading, the first words
    // open the page, and only they do.
    let opened = walk("# Title\n\nThe lede.\n\n## Summary\n\nFirst words.\n\nMore words.\n");
    assert!(
        opened.contains("#opening[#metadata(\"s 7-7\")#\"First words.\""),
        "{opened}"
    );
    assert!(
        opened.contains("#par[#metadata(\"s 9-9\")#\"More words.\""),
        "{opened}"
    );
    assert_eq!(opened.matches("#opening[").count(), 1, "{opened}");
    // A page whose words begin with a list, a table or code has begun
    // before its first paragraph, which stands as any other.
    let listed = walk("- One\n- Two\n\nWords after.\n");
    assert!(!listed.contains("#opening["), "{listed}");
    // A quote's words are the quote's.
    let quoted = walk("> Quoted words.\n\nWords after.\n");
    assert!(!quoted.contains("#opening["), "{quoted}");
    // A picture before them is the page's plate, titled or not, and the
    // words under it open the page.
    for pictured in [
        "![A plate](plate.png)\n\nWords after.\n",
        "# Title\n\n![A plate](plate.png)\n\nWords after.\n",
    ] {
        let pictured = walk(pictured);
        assert_eq!(pictured.matches("#opening[").count(), 1, "{pictured}");
    }
}

#[test]
fn a_page_ends_with_its_mark_on_a_line_of_its_own() {
    let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
    let walk = |markdown: &str| walked(&world, markdown);
    let words = walk("First.\n\n---\n\nLast words.\n");
    assert!(words.contains("#rule()"), "{words}");
    let before = words.trim_end().strip_suffix("#the-end()");
    assert!(
        before.is_some_and(|before| before.trim_end().ends_with("#\"Last words.\"]")),
        "{words}"
    );
    let code = walk("Words.\n\n```\ncode\n```\n");
    assert!(code.trim_end().ends_with("\n#the-end()"), "{code}");
    assert_eq!(code.matches("#the-end").count(), 1, "{code}");
    assert!(!walk("").contains("#the-end"), "an empty page has no end");
}

#[test]
fn a_picture_is_known_by_its_first_bytes_not_its_name() {
    assert_eq!(picture_kind(b"\x89PNG\r\n\x1a\n...."), Some("png"));
    assert_eq!(picture_kind(b"\xff\xd8\xff\xe0...."), Some("jpg"));
    assert_eq!(picture_kind(b"GIF89a...."), Some("gif"));
    assert_eq!(picture_kind(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
    assert_eq!(picture_kind(b"%PDF-1.7\n"), Some("pdf"));
    assert_eq!(
        picture_kind(b"<?xml version=\"1.0\"?>\n<svg xmlns="),
        Some("svg")
    );
    assert_eq!(picture_kind(b"not a picture"), None);
    assert_eq!(picture_kind(b""), None);
}

#[test]
fn math_in_a_cell_is_measured_word_by_word_and_code_as_one_span() {
    let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
    let typst = walked(
        &world,
        "| Sum | Run |\n|---|---|\n| $x^2 + y^2 = z^2$ | `cargo test --lib` |\n",
    );
    // What the sheet is told of each column, before its rows.
    let from = typst
        .find("#sheet(")
        .unwrap_or_else(|| panic!("no table's sheet in {typst}"));
    let sheet = &typst[from..];
    let sheet = &sheet[..sheet.find("head:").unwrap_or(sheet.len())];
    assert!(
        sheet.contains("text(\"x^2\")") && !sheet.contains("least-code(\"x^2"),
        "math is drawn as an equation, so measured as words: {sheet}"
    );
    assert!(
        sheet.contains("least-code(\"cargo test --lib\")"),
        "code is one span, measured as the page sets it: {sheet}"
    );
}

#[test]
fn only_a_cell_of_figures_is_a_figure() {
    for figure in ["42", "3.5", "$1,200", "-4%", "12:30", "2 \u{d7} 3"] {
        assert!(is_figure(figure), "{figure} is a figure");
    }
    for words in ["none", "v2", "", "-", "12 ms"] {
        assert!(!is_figure(words), "{words} isn't a figure");
    }
}
