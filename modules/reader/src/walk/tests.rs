use super::tables::is_figure;
use super::{Costs, Pieces, picture_kind, program};
use crate::world::PageWorld;

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
    let walk = |markdown: &str| {
        program(
            markdown,
            "notes",
            None,
            &Pieces::new(),
            &world,
            crate::page::Width::whole(),
            &mut Costs::default(),
        )
    };
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
    // The last words carry the page's end mark.
    assert!(
        titled.contains("#par[#metadata(\"s 5-5\")#\"The words.\""),
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
    let walk = |markdown: &str| {
        program(
            markdown,
            "notes",
            None,
            &Pieces::new(),
            &world,
            crate::page::Width::whole(),
            &mut Costs::default(),
        )
    };
    let alone = walk("![What it shows](missing.png)\n");
    assert!(
        alone.contains("#alone[#picture(\"What it shows\")][]"),
        "one that can't be shown says its words once, with no caption: {alone}"
    );
    let among = walk("Words ![What it shows](missing.png) and more.\n");
    assert!(!among.contains("#alone"), "{among}");
    assert!(among.contains("#par["), "{among}");
}

#[test]
fn a_page_ends_with_its_mark_after_its_last_word() {
    let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
    let walk = |markdown: &str| {
        program(
            markdown,
            "notes",
            None,
            &Pieces::new(),
            &world,
            crate::page::Width::whole(),
            &mut Costs::default(),
        )
    };
    let words = walk("First.\n\n---\n\nLast words.\n");
    assert!(words.contains("#rule()"), "{words}");
    assert!(
        words
            .trim_end()
            .ends_with("#\"Last words.\"#h(0.3em)#the-end]"),
        "{words}"
    );
    let code = walk("Words.\n\n```\ncode\n```\n");
    assert!(code.trim_end().ends_with("#par[#the-end]"), "{code}");
    assert_eq!(code.matches("#the-end").count(), 1, "{code}");
    assert!(!walk("").contains("#the-end]"), "an empty page has no end");
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
fn only_a_cell_of_figures_is_a_figure() {
    for figure in ["42", "3.5", "$1,200", "-4%", "12:30", "2 \u{d7} 3"] {
        assert!(is_figure(figure), "{figure} is a figure");
    }
    for words in ["none", "v2", "", "-", "12 ms"] {
        assert!(!is_figure(words), "{words} isn't a figure");
    }
}
