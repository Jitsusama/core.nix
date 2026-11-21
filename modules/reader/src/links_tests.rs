//! The tests of the page's links: what gf and Ctrl+] take under or after
//! the cursor's character, where a link goes, and how links are labelled.

use super::*;

#[test]
fn the_name_under_the_character_or_the_next_after_it_on_its_line_is_taken() {
    // Two links on a line set on 40 pt and running to 300 pt across,
    // given out of their order, one past its end on its baseline, as
    // in a table's next cell, and one on the line under it.
    let named = || {
        [
            (41.5, 100.0, 160.0, "second"),
            (40.0, 10.0, 60.0, "first"),
            (40.0, 320.0, 360.0, "next cell"),
            (61.0, 0.0, 300.0, "under"),
        ]
    };
    let mut line = crate::text::test_line(40.0, "words");
    line.spans = vec![(0.0, 1.0), (1.0, 2.0), (2.0, 3.0), (3.0, 4.0), (4.0, 300.0)];
    assert_eq!(under_or_after(named(), &line, 30.0), Some("first"), "on it");
    assert_eq!(
        under_or_after(named(), &line, 60.0),
        Some("second"),
        "at the first's very end, the next, a superscript's nudge off"
    );
    assert_eq!(
        under_or_after(named(), &line, 5.0),
        Some("first"),
        "before them all, the least across, not the first given"
    );
    assert_eq!(
        under_or_after(named(), &line, 170.0),
        None,
        "past the line's last, none, though the next cell and a line below have one"
    );
    let together = [(40.0, 10.2, 60.0, "link"), (40.0, 10.0, 60.0, "path")];
    assert_eq!(
        under_or_after(together, &line, 30.0),
        Some("link"),
        "of two starting together, the first given"
    );
}

/// A page set from `markdown`, its lines as the keys read them and the
/// pieces of its links.
fn set(name: &str, markdown: &str) -> (Vec<crate::text::Line>, Vec<Link>) {
    let dir = std::env::temp_dir().join(format!("reader-links-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(&file, markdown).expect("the file");
    let (page, _) = crate::page::Setter::new(file.to_str().expect("a path"))
        .set()
        .expect("set");
    std::fs::remove_dir_all(&dir).expect("cleaned");
    let layout = crate::select::Layout::read(&page.frame, markdown.to_string(), 0);
    (layout.lines, pieces(&page.frame))
}

/// The line that has `words`, and the middle of the character `at`
/// characters into them, as the cursor's character gives it.
fn character<'a>(
    lines: &'a [crate::text::Line],
    words: &str,
    at: usize,
) -> (&'a crate::text::Line, f64) {
    let texts: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
    let line = lines
        .iter()
        .find(|line| line.text.contains(words))
        .unwrap_or_else(|| panic!("no line has {words:?}: {texts:?}"));
    let ch = line.text[..line.text.find(words).expect("found")]
        .chars()
        .count()
        + at;
    let x_pt = crate::text::middle_of(line, ch).expect("a character there");
    (line, x_pt)
}

/// The name gf takes with the character `at` characters into `words`,
/// every path written there being a file.
fn taken(lines: &[crate::text::Line], pieces: &[Link], words: &str, at: usize) -> String {
    let (line, x_pt) = character(lines, words, at);
    name_here(pieces, line, x_pt, |_| true).map_or_else(
        || format!("nothing at {words:?}"),
        |name| name.said().to_string(),
    )
}

/// What gf opens, or says, with the character `at` characters into
/// `words` on a page in /notes, whose folder holds only `there`.
fn opened(
    lines: &[crate::text::Line],
    pieces: &[Link],
    (words, at): (&str, usize),
    there: &[&str],
) -> String {
    let (line, x_pt) = character(lines, words, at);
    let page = Path::new("/notes/page.md");
    let is_file = |file: &Path| {
        file.strip_prefix("/notes")
            .is_ok_and(|name| there.iter().any(|one| name == Path::new(one)))
    };
    match file_here(pieces, line, x_pt, page, is_file) {
        Ok(file) => file.display().to_string(),
        Err(Some(said)) => format!("E447 {said}"),
        Err(None) => "E446".to_string(),
    }
}

/// What Ctrl+] follows with the character `at` characters into `words`.
fn followed(lines: &[crate::text::Line], pieces: &[Link], words: &str, at: usize) -> String {
    let (line, x_pt) = character(lines, words, at);
    link_here(pieces, line, x_pt)
        .map_or_else(|| format!("nothing at {words:?}"), |link| link.href.clone())
}

#[test]
fn a_character_on_a_lines_second_link_takes_that_one() {
    let (lines, pieces) = set(
        "two-links",
        "Read [the one](one.md) and then [the other](other.md) here, or notes.md.\n",
    );
    assert_eq!(taken(&lines, &pieces, "the other", 4), "other.md", "on it");
    assert_eq!(
        followed(&lines, &pieces, "the other", 4),
        "other.md",
        "Ctrl+] too"
    );
    assert_eq!(
        followed(&lines, &pieces, "here", 0),
        "nothing at \"here\"",
        "Ctrl+] past the last link, nothing, not a link behind"
    );
    assert_eq!(
        taken(&lines, &pieces, "the one", 4),
        "one.md",
        "on the first"
    );
    assert_eq!(
        taken(&lines, &pieces, "and then", 0),
        "other.md",
        "between them, the one after"
    );
    assert_eq!(
        taken(&lines, &pieces, "here", 0),
        "notes.md",
        "past the last link, the path written after, never a link behind"
    );
    assert_eq!(
        taken(&lines, &pieces, "notes.md", 8),
        "nothing at \"notes.md\"",
        "on the full stop past it all, nothing"
    );
}

#[test]
fn a_character_on_a_broken_links_second_line_takes_that_link() {
    let (lines, pieces) = set(
        "broken-link",
        &format!(
            "{}[a link long enough to run on past the end of its first line and onto the next one, so it breaks](https://example.com) and [a second](#here).\n",
            "Words before it. ".repeat(5)
        ),
    );
    let ends = lines
        .iter()
        .find(|line| line.text.contains("so it"))
        .expect("the link's end is set");
    assert!(
        !ends.text.contains("a link long"),
        "the link ends on a line below its start: {:?}",
        ends.text
    );
    assert_eq!(
        taken(&lines, &pieces, "so it", 0),
        "https://example.com",
        "the link the character is on, not the next"
    );
    assert_eq!(
        followed(&lines, &pieces, "so it", 0),
        "https://example.com",
        "Ctrl+] too"
    );
}

#[test]
fn a_link_whose_label_is_a_path_is_taken_over_the_path() {
    let (lines, pieces) = set("labelled", "See [docs/design.md](other.md) first.\n");
    assert_eq!(taken(&lines, &pieces, "docs/design.md", 0), "other.md");
    assert_eq!(taken(&lines, &pieces, "docs/design.md", 6), "other.md");
}

#[test]
fn the_name_under_the_character_is_taken_though_it_names_no_file_here() {
    let (lines, pieces) = set(
        "no-file",
        "See [the site](https://example.com) or [old](gone.md), or notes.md.\n",
    );
    // gf settles the name and only then asks the disk, so it says it
    // can't find these rather than opening notes.md.
    let there = ["notes.md"];
    assert_eq!(
        opened(&lines, &pieces, ("the site", 4), &there),
        "E447 https://example.com"
    );
    assert_eq!(opened(&lines, &pieces, ("old", 0), &there), "E447 gone.md");
    assert_eq!(
        opened(&lines, &pieces, ("or notes", 0), &there),
        "/notes/notes.md"
    );
    assert_eq!(
        opened(&lines, &pieces, ("notes.md", 8), &there),
        "E446",
        "past it all, no name"
    );
    let page = Path::new("/notes/page.md");
    assert_eq!(
        file_of(&Name::Link("#a-heading".into()), page),
        Some(page.to_path_buf()),
        "a heading here names this page, which is there"
    );
    if let Some(home) = std::env::var_os("HOME") {
        assert_eq!(
            file_of(&Name::Written("~/notes.md".into()), page),
            Some(PathBuf::from(home).join("notes.md")),
            "a written path from ~/ is from home"
        );
    }
    assert_eq!(
        file_of(&Name::Link("https://example.com".into()), page),
        None,
        "a site's address names no file"
    );
    assert_eq!(
        file_of(&Name::Link("gone.md".into()), page),
        Some(PathBuf::from("/notes/gone.md")),
        "a link names its file whether or not it's there"
    );
    assert_eq!(
        file_of(&Name::Written("notes.md".into()), page),
        Some(PathBuf::from("/notes/notes.md")),
        "a written path is from the page's folder"
    );
}

#[test]
fn a_word_read_as_a_path_after_the_character_stops_gf_only_when_its_a_file() {
    let (lines, pieces) = set(
        "dotted",
        "Read e.g. and/or the [guide](guide.md), or notes.md.\n",
    );
    assert_eq!(
        opened(&lines, &pieces, ("Read", 0), &["guide.md"]),
        "/notes/guide.md",
        "past words that aren't files, to the link"
    );
    assert_eq!(
        opened(&lines, &pieces, ("e.g.", 0), &["guide.md"]),
        "E447 e.g",
        "under the character, the word as written"
    );
    assert_eq!(
        opened(&lines, &pieces, ("Read", 0), &["guide.md", "and/or"]),
        "/notes/and/or",
        "a written path that is a file comes first"
    );
}

#[test]
fn in_a_table_a_character_takes_only_from_its_own_cell() {
    let (lines, pieces) = set(
        "table-row",
        "| a | b | c |\n|---|---|---|\n| see below | notes.md | [spec](spec.md) |\n",
    );
    let (line, _) = character(&lines, "see below", 0);
    assert!(
        !line.text.contains("notes.md"),
        "a row's cells are lines of their own: {:?}",
        line.text
    );
    assert_eq!(
        taken(&lines, &pieces, "see below", 4),
        "nothing at \"see below\"",
        "not the next cell's link, past the path in the cell between"
    );
    assert_eq!(taken(&lines, &pieces, "notes.md", 0), "notes.md");
    assert_eq!(taken(&lines, &pieces, "spec", 0), "spec.md");
    assert_eq!(followed(&lines, &pieces, "spec", 0), "spec.md");
}

#[test]
fn a_paths_escapes_are_read_back() {
    let page = Path::new("/notes/page.md");
    assert_eq!(
        target("/notes/a%20b/%5Bold%5D.md", page),
        Target::Page(PathBuf::from("/notes/a b/[old].md"), None)
    );
    assert_eq!(unescaped("100%"), "100%");
    assert_eq!(unescaped("caf%C3%A9"), "caf\u{e9}");
}

#[test]
fn a_link_goes_here_by_its_fragment_and_elsewhere_otherwise() {
    let page = Path::new("/notes/page.md");
    assert_eq!(target("#the-keys", page), Target::Here("the-keys".into()));
    assert_eq!(
        target("https://example.com/a#b", page),
        Target::Elsewhere("https://example.com/a#b".into())
    );
    assert_eq!(
        target("mailto:joel@example.com", page),
        Target::Elsewhere("mailto:joel@example.com".into())
    );
    assert_eq!(
        target("other.md", page),
        Target::Page("/notes/other.md".into(), None)
    );
    assert_eq!(
        target("../up.MARKDOWN#top", page),
        Target::Page("/notes/../up.MARKDOWN".into(), Some("top".into()))
    );
    assert_eq!(
        target("page.md#the-keys", page),
        Target::Here("the-keys".into())
    );
    assert_eq!(target("page.md", page), Target::Here(String::new()));
    assert_eq!(
        target("sheet.pdf", page),
        Target::Elsewhere("/notes/sheet.pdf".into())
    );
    assert_eq!(
        target("/etc/hosts", page),
        Target::Elsewhere("/etc/hosts".into())
    );
}

#[test]
fn a_headings_anchor_is_githubs() {
    assert_eq!(slug("Vi's Keys"), "vis-keys");
    assert_eq!(
        slug("  Every Pixel Accounted For "),
        "every-pixel-accounted-for"
    );
    assert_eq!(slug("A, B and C: done"), "a-b-and-c-done");
    assert_eq!(
        slug("snake_case and kebab-case"),
        "snake_case-and-kebab-case"
    );
}

#[test]
fn labels_are_one_letter_while_they_last_and_none_begins_another() {
    assert_eq!(labels(3), ["a", "s", "d"]);
    let many = labels(30);
    assert_eq!(many.len(), 30);
    assert!(
        many.iter().all(|label| label.len() == 2),
        "thirty labels weren't all two letters: {many:?}"
    );
    for (i, a) in many.iter().enumerate() {
        for b in &many[i + 1..] {
            assert!(
                !b.starts_with(a.as_str()) && !a.starts_with(b.as_str()),
                "one of {a} and {b} begins the other"
            );
        }
    }
}

#[test]
fn a_typed_label_is_chosen_waited_on_or_gone() {
    let mut one = Labelling::new(3);
    assert_eq!(one.key('s'), Typed::Chosen(1));
    let mut none = Labelling::new(3);
    assert_eq!(none.key('z'), Typed::Gone);
    let mut two = Labelling::new(30);
    assert_eq!(two.key('a'), Typed::Waiting);
    assert_eq!(two.key('d'), Typed::Chosen(2));
}

#[test]
fn a_headings_line_reads_as_its_anchor() {
    let dir = std::env::temp_dir().join(format!("reader-anchor-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(
        &file,
        "# A Title Long Enough to Run Over Onto a Second Line, However Wide Its Face Is\n\nWords.\n\n## Vi's Keys\n",
    )
    .expect("the file");
    let (page, _) = crate::page::Setter::new(file.to_str().expect("a path"))
        .set()
        .expect("set");
    std::fs::remove_dir_all(&dir).expect("cleaned");
    let lines = crate::text::lines(crate::text::runs(&page.frame));
    let texts: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
    let title = heading_at(
        &lines,
        "a-title-long-enough-to-run-over-onto-a-second-line-however-wide-its-face-is",
    );
    let keys = heading_at(&lines, "vis-keys");
    assert!(title.is_some() && keys.is_some(), "{texts:?}");
    assert!(title < keys, "{title:?} {keys:?}");
    assert_eq!(heading_at(&lines, "words-vis"), None);
    // A title is never hyphenated, however it wraps.
    assert!(
        lines.iter().all(|line| !line.text.contains('\u{ad}')),
        "{texts:?}"
    );
}

#[test]
fn a_link_broken_across_lines_is_labelled_once_where_it_starts() {
    let markdown = format!(
        "{}[a link long enough to run on past the end of its first line and onto the next one, so it breaks](https://example.com) and [a second](#here).\n",
        "Words before it. ".repeat(5)
    );
    let dir = std::env::temp_dir().join(format!("reader-links-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(&file, markdown).expect("the file");
    let (page, _) = crate::page::Setter::new(file.to_str().expect("a path"))
        .set()
        .expect("set");
    std::fs::remove_dir_all(&dir).expect("cleaned");
    let found = links(&page.frame);
    let hrefs: Vec<&str> = found.iter().map(|link| link.href.as_str()).collect();
    assert_eq!(hrefs, ["https://example.com", "#here"]);
}
