use super::{Setter, Width};
use crate::numbers::i32_of;

/// The measure and spread a window this many points wide sets.
fn width(window_pt: f64) -> (f64, f64) {
    let Width {
        measure_pt,
        spread_pt,
    } = Width::for_window(window_pt);
    (measure_pt, spread_pt)
}

#[test]
fn a_window_too_narrow_for_the_whole_page_gives_way_measure_then_margin() {
    // The whole page: the measure, its outer column and xxl either side.
    let spread = super::spread_pt();
    let whole = 435.0 + 2.0 * (spread + 24.0);
    assert_eq!(width(whole), (435.0, spread), "the whole page, just");
    assert_eq!(
        width(whole - 1.0),
        (435.0, 0.0),
        "no spread, the whole measure"
    );
    assert_eq!(
        width(483.0),
        (435.0, 0.0),
        "the whole measure, xxl either side"
    );
    assert_eq!(
        width(360.0),
        (312.0, 0.0),
        "a third of the panel: xxl either side"
    );
    assert_eq!(width(348.0), (300.0, 0.0), "45 characters, xxl either side");
    assert_eq!(
        width(340.0),
        (300.0, 0.0),
        "45 characters, the margins giving way"
    );
    assert_eq!(width(336.0), (300.0, 0.0), "45 characters, xl either side");
    assert_eq!(
        width(300.0),
        (264.0, 0.0),
        "under 45 characters, xl either side"
    );
}

#[test]
fn a_window_wider_than_the_whole_page_widens_the_measure_by_half_up_to_80_characters() {
    let spread = super::spread_pt();
    let whole = 435.0 + 2.0 * (spread + 24.0);
    let character = 435.0 / 66.0;
    let widest = 435.0 * 80.0 / 66.0;
    let near = |window_pt: f64, measure_pt: f64| {
        let (measure, spread_at) = width(window_pt);
        (measure - measure_pt).abs() < 0.001 && (spread_at - spread).abs() < f64::EPSILON
    };
    assert!(
        near(whole + 2.0 * 3.0 * character - 1.0, 435.0),
        "under three characters' widening, none"
    );
    assert!(
        near(
            whole + 2.0 * 3.0 * character + 0.01,
            435.0 + 3.0 * character
        ),
        "half of what it has past, three characters at a time"
    );
    assert!(
        near(whole + 2.0 * 9.0 * character + 5.0, 435.0 + 9.0 * character),
        "nine characters"
    );
    assert_eq!(
        width(whole + 2.0 * (widest - 435.0) + 10.0),
        (widest, spread),
        "80 characters at most"
    );
    assert_eq!(width(2560.0), (widest, spread), "however wide");
    // A hyphen costs no less on a wider line than on the whole measure.
    assert!((Width::for_window(2560.0).hyphen_cost() - 200.0).abs() < f64::EPSILON);
    assert!(Width::for_window(360.0).hyphen_cost() > 200.0);
}

#[test]
fn a_page_set_for_a_wide_window_is_its_widened_measure_wide() {
    let dir = std::env::temp_dir().join(format!("reader-wide-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(&file, format!("{OPENING}{OPENING}")).expect("the file");
    let mut setter = Setter::new(file.to_str().expect("a path"));
    let wide = Width::for_window(2560.0);
    setter.fit(wide);
    let (page, _) = setter.set().expect("set");
    std::fs::remove_dir_all(&dir).expect("cleaned");
    assert!(
        (page.frame.size().x.to_pt() - wide.page_pt()).abs() < 0.01,
        "the page is {} pt wide, not {} pt",
        page.frame.size().x.to_pt(),
        wide.page_pt()
    );
    // How far across the measure the words reach, from its left edge.
    let widest_line = crate::text::lines(crate::text::runs(&page.frame))
        .iter()
        .filter_map(|line| line.spans.last().map(|span| span.1 - wide.spread_pt))
        .fold(0.0, f64::max);
    assert!(
        widest_line > 440.0 && widest_line <= wide.measure_pt + 0.5,
        "the widest line is {widest_line} pt on a {} pt measure",
        wide.measure_pt
    );
}

#[test]
fn a_page_set_for_a_narrow_window_is_its_measure_wide_and_keeps_to_it() {
    let dir = std::env::temp_dir().join(format!("reader-narrow-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(&file, OPENING).expect("the file");
    let mut setter = Setter::new(file.to_str().expect("a path"));
    setter.fit(Width::for_window(360.0));
    let (page, _) = setter.set().expect("set");
    std::fs::remove_dir_all(&dir).expect("cleaned");
    assert!(
        (page.frame.size().x.to_pt() - 312.0).abs() < 0.01,
        "the page is {} pt wide, not its measure",
        page.frame.size().x.to_pt()
    );
    let runs = crate::text::runs(&page.frame);
    let right = runs
        .iter()
        .flat_map(|run| run.chars.iter().map(|c| c.2))
        .fold(0.0, f64::max);
    assert!(
        right <= 312.01,
        "a line runs to {right} pt, past the measure"
    );
    let mut lines: Vec<f64> = runs.iter().map(|run| run.baseline_pt).collect();
    lines.sort_by(f64::total_cmp);
    lines.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    assert!(
        lines.len() > 2,
        "the words never wrapped on the narrow measure"
    );
}

/// How far across the page each shape it draws reaches, from its left
/// edge to its right, stepping into every group it's set in.
fn shapes_across(frame: &typst::layout::Frame, left: f64, out: &mut Vec<(f64, f64)>) {
    for (pos, item) in frame.items() {
        let x = left + pos.x.to_pt();
        match item {
            typst::layout::FrameItem::Group(group) => {
                shapes_across(&group.frame, x, out);
            }
            typst::layout::FrameItem::Shape(shape, _) => {
                let bounds = shape.bbox(true);
                out.push((x + bounds.min.x.to_pt(), x + bounds.max.x.to_pt()));
            }
            _ => {}
        }
    }
}

#[test]
fn a_narrow_pages_code_keeps_its_ground_on_the_page() {
    // A narrow window's page is its measure alone, with no margin for a
    // block's ground to hang into, so a short block's ground keeps to
    // the measure rather than being cut flush with its words.
    let dir = std::env::temp_dir().join(format!("reader-narrow-code-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(&file, format!("{OPENING}```\nlet code = here;\n```\n")).expect("the file");
    let mut setter = Setter::new(file.to_str().expect("a path"));
    setter.fit(Width::for_window(360.0));
    let (page, _) = setter.set().expect("set");
    std::fs::remove_dir_all(&dir).expect("cleaned");
    let wide = page.frame.size().x.to_pt();
    let mut shapes = Vec::new();
    shapes_across(&page.frame, 0.0, &mut shapes);
    assert!(!shapes.is_empty(), "the code block drew no ground");
    for (left, right) in shapes {
        assert!(
            left >= -0.01 && right <= wide + 0.01,
            "a ground runs from {left} pt to {right} pt, off the {wide} pt page"
        );
    }
}

#[test]
fn a_narrow_page_hyphenates_as_a_typesetter_would() {
    // Every narrow window from 180 to 360 pt, so the rules hold at any
    // measure rather than at one that happens to break well.
    let markdown = "When you use two or more headings, GitHub automatically generates a table of contents that you can access by clicking the \"Outline\" menu icon {% octicon \"list-unordered\" aria-label=\"Table of Contents\" %} within the file header. Each heading title is listed in the table of contents and you can click a title to navigate to the selected section.\n";
    let dir = std::env::temp_dir().join(format!("reader-hyphens-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(&file, markdown).expect("the file");
    let mut setter = Setter::new(file.to_str().expect("a path"));
    let mut wrong = Vec::new();
    for window in (180..=360).step_by(6) {
        setter.fit(Width::for_window(f64::from(window)));
        let (page, _) = setter.set().expect("set");
        let lines: Vec<String> = crate::text::lines(crate::text::runs(&page.frame))
            .into_iter()
            .map(|line| line.text)
            .collect();
        let broken: Vec<bool> = lines.iter().map(|line| line.ends_with('\u{ad}')).collect();
        if broken.windows(3).any(|three| three.iter().all(|b| *b)) {
            wrong.push(format!("{window} pt: three hyphens in a row in {lines:?}"));
        }
        for (line, next) in lines.iter().zip(lines.iter().skip(1)) {
            let head = line.rsplit(' ').next().unwrap_or_default();
            let tail = next.split(' ').next().unwrap_or_default();
            if line.ends_with('\u{ad}') && (head.contains('-') || tail.contains('-')) {
                wrong.push(format!(
                    "{window} pt: {head:?} {tail:?} broken off its hyphen"
                ));
            }
        }
    }
    std::fs::remove_dir_all(&dir).expect("cleaned");
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Every baseline from the first run holding `from` to the last holding
/// `to`, top to bottom, and the page's pitch from its first paragraph,
/// which is the page's line.
/// The page a file holding `markdown` sets, as the window's is.
fn page_of(name: &str, markdown: &str) -> super::Page {
    let dir = std::env::temp_dir().join(format!("reader-pitch-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(&file, markdown).expect("the file");
    let (page, _) = Setter::new(file.to_str().expect("a path"))
        .set()
        .expect("set");
    std::fs::remove_dir_all(&dir).expect("cleaned");
    page
}

fn baselines(name: &str, markdown: &str, from: &str, to: &str) -> (f64, Vec<f64>) {
    let page = page_of(name, markdown);
    let runs = crate::text::runs(&page.frame);
    let text = |run: &crate::text::Run| run.chars.iter().map(|c| c.0).collect::<String>();
    let mut all: Vec<f64> = runs.iter().map(|run| run.baseline_pt).collect();
    all.sort_by(f64::total_cmp);
    all.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    // The opening paragraph's first two lines, past the eyebrow and any
    // title over it.
    let opening = runs
        .iter()
        .filter(|run| text(run).contains("opening"))
        .map(|run| run.baseline_pt)
        .fold(f64::INFINITY, f64::min);
    let next = all
        .iter()
        .find(|b| **b > opening + 0.01)
        .expect("a second line");
    let pitch = next - opening;
    assert!(
        (pitch - super::pitch_pt()).abs() < 0.01,
        "the words step {pitch} pt, not the page's line of {} pt",
        super::pitch_pt()
    );
    let top = runs
        .iter()
        .filter(|run| text(run).contains(from))
        .map(|run| run.baseline_pt)
        .fold(f64::INFINITY, f64::min);
    let bottom = runs
        .iter()
        .filter(|run| text(run).contains(to))
        .map(|run| run.baseline_pt)
        .fold(f64::NEG_INFINITY, f64::max);
    (
        pitch,
        all.into_iter()
            .filter(|b| *b >= top - 0.01 && *b <= bottom + 0.01)
            .collect(),
    )
}

/// Each step between baselines, in hundredths of a pitch.
fn steps(pitch: f64, lines: &[f64]) -> Vec<i64> {
    lines
        .windows(2)
        .map(|pair| i64::from(i32_of(((pair[1] - pair[0]) / pitch * 100.0).round())))
        .collect()
}

const OPENING: &str = "The opening paragraph runs long enough to wrap onto a second line and a third, so the page's pitch can be read from its first two baselines without guessing at what Typst set.\n\n";

#[test]
fn the_words_land_on_the_pitch_a_paragraph_one_and_a_half() {
    let markdown = format!(
        "{OPENING}The second paragraph runs long enough to wrap onto a second line and a third as well, so a step between paragraphs sits among steps between lines, whichever face and size the page sets its words in, plain or the theme's, before it comes to an end at Omega.\n"
    );
    let (pitch, lines) = baselines("words", &markdown, "opening", "Omega");
    let steps = steps(pitch, &lines);
    assert!(steps.len() >= 4, "too few lines: {steps:?}");
    assert_eq!(
        steps.iter().filter(|s| **s == 150).count(),
        1,
        "one step between paragraphs, of one and a half lines: {steps:?}"
    );
    assert!(
        steps.iter().all(|s| *s == 100 || *s == 150),
        "a line steps one pitch, a paragraph one and a half: {steps:?}"
    );
}

#[test]
fn a_page_ends_three_lines_under_its_last_words() {
    let page = page_of("foot", &format!("{OPENING}Omega.\n"));
    let last = crate::text::runs(&page.frame)
        .iter()
        .map(|run| run.baseline_pt)
        .fold(f64::NEG_INFINITY, f64::max);
    // The end's mark stands two lines under the last words, as a break
    // does, and the page ends a line under it.
    let foot = page.frame.height().to_pt() - last;
    assert!(
        (foot - 3.0 * super::pitch_pt()).abs() < 0.01,
        "the page ends {foot} pt under its last words, not three lines of {} pt",
        super::pitch_pt()
    );
}

#[test]
fn a_pages_first_capitals_stand_a_line_under_its_top() {
    let program = super::at_width("", "", super::Width::whole());
    let line = super::pitch_pt();
    assert!(
        program.contains(&format!("top: {line}pt, bottom: {line}pt")),
        "the eyebrow's capitals a line under the top and the last baseline a line over \
             the foot: {program}"
    );
}

#[test]
fn code_sets_its_words_on_the_column_the_paragraphs_start_at() {
    let markdown = format!("{OPENING}```\nlet code = here;\n```\n");
    let page = page_of("code", &markdown);
    let runs = crate::text::runs(&page.frame);
    let start = |word: &str| {
        runs.iter()
            .filter(|run| {
                run.chars
                    .iter()
                    .map(|c| c.0)
                    .collect::<String>()
                    .contains(word)
            })
            .map(|run| run.x_pt)
            .fold(f64::INFINITY, f64::min)
    };
    let (words, code) = (start("opening"), start("let"));
    assert!(
        (words - code).abs() < 0.01,
        "code's words start at {code} pt, off the column at {words} pt"
    );
}

#[test]
fn a_line_holding_a_key_steps_one_line_as_any_other() {
    let markdown = format!(
        "{OPENING}Press <kbd>Ctrl</kbd>+<kbd>O</kbd> to go back to where you were, and <kbd>Ctrl</kbd>+<kbd>I</kbd> to come forward again, in a paragraph long enough to wrap onto a second line and a third, whichever face it's set in, before it ends at Omega.\n"
    );
    let (pitch, lines) = baselines("keys", &markdown, "opening", "Omega");
    let steps = steps(pitch, &lines);
    assert!(steps.len() >= 4, "too few lines: {steps:?}");
    assert!(
        steps.iter().all(|s| *s == 100 || *s == 150),
        "a key's frame grew its line: {steps:?}"
    );
}

#[test]
fn a_line_all_of_code_steps_one_line_as_any_other() {
    // The mono's capitals stand at their own height, not the words', so a
    // line wrapped inside a long piece of code, with no words on it, once
    // stepped its own distance and put every line after it off the grid.
    let command = "nix-offload there --here-when-idle . checks.x86_64-linux.reader-sets-its-pages";
    let markdown = format!(
        "{OPENING}Run `{command} {command} {command}` to check it, in a paragraph that ends at Omega.\n"
    );
    let (pitch, lines) = baselines("code-lines", &markdown, "opening", "Omega");
    let steps = steps(pitch, &lines);
    // The opening's two lines or more, and three for the code, of which
    // its three commands fill one whole.
    assert!(steps.len() >= 4, "too few lines: {steps:?}");
    let set = crate::text::lines(crate::text::runs(&page_of("code-lines", &markdown).frame));
    assert!(
        set.iter().any(|line| !line.text.trim().is_empty()
            && format!("{command} {command} {command}").contains(line.text.trim())),
        "no line was all code: {:?}",
        set.iter().map(|line| &line.text).collect::<Vec<_>>()
    );
    assert!(
        steps.iter().all(|s| *s == 100 || *s == 150),
        "a line of code stepped its own distance: {steps:?}"
    );
}

/// Each piece of text the page draws, with the family of the face it's
/// drawn in, stepping into every group it's set in.
fn faces_of(frame: &typst::layout::Frame, out: &mut Vec<(String, String)>) {
    for (_, item) in frame.items() {
        match item {
            typst::layout::FrameItem::Group(group) => faces_of(&group.frame, out),
            typst::layout::FrameItem::Text(text) => {
                out.push((text.text.to_string(), text.font.info().family.clone()));
            }
            _ => {}
        }
    }
}

#[test]
fn an_emoji_is_drawn_in_the_colour_face_and_keeps_its_line() {
    let markdown = format!(
        "{OPENING}A party 🎉, a thumb 👍🏽, a flag 🇨🇦, a coder 👩\u{200d}💻 and a warning ⚠\u{fe0f} sit in a line, an arrow ↔ and a mark ™ stay words, in a paragraph long enough to wrap, before it ends at Omega.\n"
    );
    let (pitch, lines) = baselines("emoji", &markdown, "opening", "Omega");
    let steps = steps(pitch, &lines);
    assert!(steps.len() >= 3, "too few lines: {steps:?}");
    assert!(
        steps.iter().all(|s| *s == 100 || *s == 150),
        "an emoji's taller face grew its line: {steps:?}"
    );
    let mut faces = Vec::new();
    faces_of(&page_of("emoji-faces", &markdown).frame, &mut faces);
    let face = |wanted: &str| {
        faces
            .iter()
            .find(|(text, _)| text.contains(wanted))
            .map(|(_, family)| family.clone())
    };
    for emoji in ["🎉", "👍🏽", "🇨🇦", "👩\u{200d}💻", "⚠\u{fe0f}"] {
        assert_eq!(
            face(emoji).as_deref(),
            Some("Noto Color Emoji"),
            "{emoji} is drawn in another face, or the tests have no colour face: they \
             read it from READER_FONTS, as package.nix's preCheck sets it: {faces:?}"
        );
    }
    // The colour face draws both as pictures too, so neither would stay a
    // word were the emoji's rule to take them.
    for words in ["↔", "™"] {
        assert_ne!(
            face(words).as_deref(),
            Some("Noto Color Emoji"),
            "{words} is drawn as a picture: {faces:?}"
        );
    }
}

#[test]
fn a_long_opening_drops_its_capital_and_reads_as_it_did() {
    let long = "The opening paragraph of this page runs long enough to wrap onto a second line, a third and a fourth at the measure, however wide its face sets, so that it opens on a letter dropped two lines down beside its first words, which then run on at the whole measure under it to the end, at Omega.\n";
    let page = page_of("drop", long);
    let runs = crate::text::runs(&page.frame);
    let words = runs
        .iter()
        .filter(|run| run.chars.len() > 2)
        .map(|run| run.size_pt)
        .fold(0.0, f64::max);
    let (letter, letter_baseline, letter_cap) = runs
        .iter()
        .find(|run| run.size_pt > 2.0 * words)
        .map(|run| {
            (
                run.chars.iter().map(|c| c.0).collect::<String>(),
                run.baseline_pt,
                run.cap_pt,
            )
        })
        .expect("no letter was dropped");
    assert_eq!(letter, "T");
    // Past the eyebrow, the opening's lines, read as they were written.
    let lines: Vec<_> = crate::text::lines(runs)
        .into_iter()
        .skip_while(|line| !line.text.contains("opening"))
        .collect();
    assert!(
        lines[0].text.starts_with("The opening paragraph"),
        "the first line reads {:?}",
        lines[0].text
    );
    // Every word once, in its place, across the cut between the lines
    // beside the letter and the rest.
    let read = lines
        .iter()
        .map(|line| line.text.trim_end())
        .collect::<Vec<_>>()
        .join(" ")
        .replace("\u{ad} ", "");
    assert_eq!(read, long.trim_end(), "the opening reads otherwise");
    // Two lines stand beside the letter, and the third is back at its
    // edge, under it.
    let edge = lines[0].x_pt;
    assert!(
        lines[1].x_pt > edge + 10.0 && (lines[2].x_pt - edge).abs() < 0.01,
        "the lines start at {:?}",
        lines.iter().map(|line| line.x_pt).collect::<Vec<_>>()
    );
    // The letter's capitals stand from the first line's to the second's
    // baseline, on which it sits.
    let pitch = super::pitch_pt();
    assert!(
        (letter_baseline - lines[1].baseline_pt).abs() < 0.01,
        "the letter sits on {letter_baseline} pt, not the second line's {} pt",
        lines[1].baseline_pt
    );
    assert!(
        (letter_cap - (pitch + lines[0].cap_pt)).abs() < 1.0,
        "its capitals stand {letter_cap} pt, not two lines' {} pt",
        pitch + lines[0].cap_pt
    );
    let steps = steps(
        pitch,
        &lines
            .iter()
            .map(|line| line.baseline_pt)
            .collect::<Vec<_>>(),
    );
    assert!(steps.len() >= 3, "too few lines: {steps:?}");
    assert!(
        steps.iter().all(|s| *s == 100),
        "the opening's lines step one apart: {steps:?}"
    );
    // One of two lines keeps its own small letter.
    let short = page_of("no-drop", "The opening is short.\n");
    let sizes: Vec<f64> = crate::text::runs(&short.frame)
        .iter()
        .map(|run| run.size_pt)
        .collect();
    assert!(
        sizes.iter().all(|size| *size < 1.5 * words),
        "a short opening dropped a letter: {sizes:?}"
    );
}

/// A page's lines, set for a window `window_pt` wide, from the first
/// holding `from` on, each without the spaces it ends on, and their
/// baselines.
fn lines_from(name: &str, markdown: &str, from: &str, window_pt: f64) -> Vec<(String, f64)> {
    let dir = std::env::temp_dir().join(format!("reader-lines-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(&file, markdown).expect("the file");
    let mut setter = Setter::new(file.to_str().expect("a path"));
    setter.fit(Width::for_window(window_pt));
    let (page, _) = setter.set().expect("set");
    std::fs::remove_dir_all(&dir).expect("cleaned");
    crate::text::lines(crate::text::runs(&page.frame))
        .into_iter()
        .skip_while(|line| !line.text.contains(from))
        .map(|line| (line.text.trim_end().to_owned(), line.baseline_pt))
        .collect()
}

#[test]
fn long_code_breaks_only_at_its_spaces_or_after_a_slash_and_never_leaves_a_scrap() {
    // Each piece narrower than two thirds of the narrowest measure here,
    // so each is kept whole; a wider one breaks where it can, as the next
    // test shows.
    let code = "nix-offload there --here-when-idle . checks.x86_64-linux.reader -L --keep-going ~/.cache/pi/agentic-harness.pi/work/trees/a-tree-with-a-long-name/tmp/runs/reader-sets-its-pages.log | tee out";
    let markdown = format!("Run this:\n\n`{code}`\n");
    let pitch = super::pitch_pt();
    let mut slashed = false;
    // Across windows from under 45 characters to the whole page, so the
    // breaks land in many places, beside each scrap among them.
    for window in (0..13).map(|i| 300.0 + 15.0 * f64::from(i)) {
        let lines = lines_from("wraps", &markdown, "nix-offload", window);
        assert!(lines.len() >= 2, "at {window} pt the code took {lines:?}");
        let texts: Vec<&str> = lines.iter().map(|(text, _)| text.as_str()).collect();
        slashed |= texts.iter().any(|text| text.ends_with('/'));
        // Every break is at a space the code has, or just after a slash.
        let broken = texts.join("\n");
        let read = broken.replace("/\n", "/").replace('\n', " ");
        assert_eq!(
            read, code,
            "at {window} pt the code broke inside a word: {texts:?}"
        );
        // A scrap under four characters keeps to the piece before it, or
        // the first of a word to the one after, so none stands alone at
        // a line's end or start.
        for cut in ["\n.", "\n-L", "~/\n", "\npi/", "\n|", "\ntee", "\nout"] {
            assert!(
                !broken.contains(cut),
                "at {window} pt a scrap stands alone, {cut:?}: {texts:?}"
            );
        }
        let baselines: Vec<f64> = lines.iter().map(|(_, baseline)| *baseline).collect();
        assert!(
            steps(pitch, &baselines).iter().all(|s| *s == 100),
            "at {window} pt the code's lines step otherwise: {baselines:?}"
        );
    }
    assert!(slashed, "no window broke the code after a slash");
}

#[test]
fn a_piece_wider_than_two_thirds_of_the_measure_breaks_where_it_can_and_keeps_to_the_page() {
    let name = ["a-name"; 30].join("-");
    let lines = lines_from(
        "wide-piece",
        &format!("Run this:\n\n`{name}`\n"),
        "a-name",
        360.0,
    );
    let measure = Width::for_window(360.0).measure_pt;
    assert!(lines.len() >= 2, "the name took {lines:?}");
    let read = lines
        .iter()
        .map(|(text, _)| text.as_str())
        .collect::<String>();
    assert_eq!(read, name, "the name reads otherwise");
    for (text, _) in &lines {
        assert!(
            text.chars().count() < name.chars().count(),
            "a line held the whole name on a {measure} pt measure: {text:?}"
        );
    }
}

#[test]
fn a_line_all_of_short_code_steps_one_line_as_any_other() {
    let markdown = format!("{OPENING}- `nix fmt`\n- `cargo test`\n- `just Omega`\n");
    let (pitch, lines) = baselines("short-code", &markdown, "nix fmt", "Omega");
    assert_eq!(lines.len(), 3, "the items took {lines:?}");
    assert!(
        steps(pitch, &lines).iter().all(|s| *s == 100),
        "a line of short code steps otherwise: {lines:?}"
    );
}

#[test]
fn code_in_a_narrow_column_keeps_to_it() {
    let markdown = format!(
        "{OPENING}| Alpha | Command | Wraps |\n|---|---|---|\n| one | `cargo build --release` | a cell long enough that it wraps onto a second line in its column, however narrow the face its words are set in, and longer still so it squeezes the others |\n"
    );
    let page = {
        let dir = std::env::temp_dir().join(format!("reader-cell-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, &markdown).expect("the file");
        let mut setter = Setter::new(file.to_str().expect("a path"));
        setter.fit(Width::for_window(2560.0));
        let (page, _) = setter.set().expect("set");
        std::fs::remove_dir_all(&dir).expect("cleaned");
        page
    };
    let runs = crate::text::runs(&page.frame);
    // Where the third column's words start: no glyph of the code may
    // reach it.
    let third = runs
        .iter()
        .filter(|run| run.chars.iter().any(|c| c.0 == 'q'))
        .map(|run| run.x_pt)
        .fold(f64::MAX, f64::min);
    let code = "cargo build --release";
    let code_end = runs
        .iter()
        .filter(|run| {
            let text: String = run.chars.iter().map(|c| c.0).collect();
            text.trim().len() >= 3 && code.contains(text.trim())
        })
        .flat_map(|run| run.chars.iter())
        .map(|c| c.2)
        .fold(0.0, f64::max);
    assert!(code_end > 0.0, "the code was never set");
    assert!(
        code_end < third,
        "the code runs to {code_end} pt, past the next column at {third} pt"
    );
}

#[test]
fn short_code_is_kept_whole_on_one_line() {
    // After an opening, so no letter is dropped beside the lines.
    let markdown = "Short.\n\nKeep it short. ".to_owned()
        + &"Never run `cargo build` here, where it evaluates beside the work. ".repeat(12)
        + "\n";
    let lines = lines_from("whole", &markdown, "Keep it", 360.0);
    assert!(lines.len() >= 6, "too few lines to show it: {lines:?}");
    let texts: Vec<&str> = lines.iter().map(|(text, _)| text.as_str()).collect();
    // Both its words are long enough to stand alone, so only the box
    // keeps them together.
    assert!(
        texts.iter().all(|text| !text.ends_with("cargo")),
        "a short span broke across lines: {texts:?}"
    );
}

/// The largest run the page sets past its eyebrow and its words, with
/// what it spells: a dropped letter, when there is one.
fn dropped(name: &str, markdown: &str) -> Option<String> {
    let runs = crate::text::runs(&page_of(name, markdown).frame);
    let words = runs
        .iter()
        .filter(|run| run.chars.len() > 2)
        .map(|run| run.size_pt)
        .fold(0.0, f64::max);
    runs.iter()
        .find(|run| run.size_pt > 2.0 * words)
        .map(|run| run.chars.iter().map(|c| c.0).collect())
}

#[test]
fn an_opening_drops_only_a_capital_and_a_letter_alone_reads_as_a_word() {
    // As long as the opening that drops its letter, so each would.
    let tail = "opening paragraph of this page runs long enough to wrap onto a second line, a third and a fourth at the measure, however wide its face sets, so that it opens on a letter dropped two lines down beside its first words, which then run on at the whole measure under it to the end, at Omega.\n";
    for (name, start) in [("small", "the"), ("strong", "**The**"), ("code", "`The`")] {
        assert_eq!(
            dropped(name, &format!("{start} {tail}")),
            None,
            "an opening on {start:?} dropped a letter"
        );
    }
    let alone = format!("A long {tail}");
    assert_eq!(dropped("alone", &alone).as_deref(), Some("A"));
    let lines: Vec<_> = crate::text::lines(crate::text::runs(&page_of("alone", &alone).frame))
        .into_iter()
        .skip_while(|line| !line.text.contains("long opening"))
        .collect();
    assert!(
        lines[0].text.starts_with("A long opening"),
        "the first line reads {:?}",
        lines[0].text
    );
}

#[test]
fn a_theme_whose_words_give_no_strong_weight_still_sets_a_strong_span() {
    let theme = crate::theme::Theme::read(
        &serde_json::json!({"levels": {"words": {"strong": null}}}).to_string(),
    );
    let mut world = super::new_world();
    let (ink, look) = super::look(&theme, &world);
    let program = format!(
        "{}Words with #strong[a strong span] in them.",
        super::at_width(&ink, &look, super::Width::whole())
    );
    let page = super::set_page(&mut world, &program).expect("the page sets");
    let words: String = crate::text::runs(&page.frame)
        .iter()
        .flat_map(|run| run.chars.iter().map(|c| c.0))
        .collect();
    assert!(words.contains("strong"), "the page says its words: {words}");
}

#[test]
fn a_pages_headings_land_on_the_half_line() {
    let markdown = format!(
        "---\nkind: page\n---\n# Title\n\nThe lede.\n\n{OPENING}## Section\n\nWords.\n\n### Part\n\nMore words.\n\n#### Fourth\n\nOmega.\n"
    );
    // The eyebrow says the kind the front matter gives.
    let (pitch, lines) = baselines("headings", &markdown, "PAGE", "Omega");
    let steps = steps(pitch, &lines);
    assert!(
        steps.iter().all(|s| s % 50 == 0),
        "every baseline lands on the half line: {steps:?}"
    );
    // The head widens outward: eyebrow to title, title to lede, lede
    // to the words, each wider than the last, the lede's widest of
    // all, past the paragraphs' one and a half.
    assert_eq!(
        steps[..3],
        [200, 150, 200],
        "the title two lines under the eyebrow, the lede one and a half under the title, \
             the words two under the lede: {steps:?}"
    );
    // A heading sits a line over its first block, nearer it than one
    // paragraph is to the next, and further from what's before it.
    assert_eq!(
        steps[steps.len() - 6..],
        [300, 100, 250, 100, 200, 100],
        "a section three lines under the words and one over its own, a part two and a \
             half and one, a fourth two and one: {steps:?}"
    );
}

#[test]
fn a_list_steps_its_items_a_line_apart_or_a_beat_further_where_one_wraps() {
    let markdown = format!(
        "{OPENING}- One\n- Two\n- Three\n\nA paragraph between.\n\n- A first item long enough that it wraps onto a second line at the measure, as a little paragraph would\n- A second\n\nOmega.\n"
    );
    let (pitch, lines) = baselines("lists", &markdown, "opening", "Omega");
    let steps = steps(pitch, &lines);
    // Each level stays wider than the one inside it: a tight list's
    // items a line apart and the list one and a half from the words
    // round it; a wrapped list's items one and a half apart, as
    // paragraphs are, and the list two off. The opening's own lines
    // come first, as many as its face wraps it to.
    assert_eq!(
        steps[steps.len() - 8..],
        [150, 100, 100, 150, 200, 100, 150, 200],
        "a tight list a line a step and one and a half off, a wrapped list one and a \
             half a step and two off: {steps:?}"
    );
}

#[test]
fn a_sections_heading_sits_a_line_over_a_list_however_its_items_wrap() {
    let markdown = format!(
        "{OPENING}## Tight\n\n- One\n- Two\n\n## Wrapped\n\n- A first item long enough that it wraps onto a second line at the measure, as a little paragraph would\n- Omega\n"
    );
    let (pitch, lines) = baselines("sectioned-lists", &markdown, "opening", "Omega");
    let steps = steps(pitch, &lines);
    // A group's heading sits a line over its first thing, a list's
    // first item as a paragraph, the list's own room giving way to it.
    assert_eq!(
        steps[steps.len() - 7..],
        [300, 100, 100, 300, 100, 100, 150],
        "each section a line over its list: {steps:?}"
    );
}

#[test]
fn a_tables_lines_land_on_the_pitch() {
    let markdown = format!(
        "{OPENING}| Alpha | Wraps | Count |\n|---|---|---:|\n| one | a cell long enough that it wraps onto a second line in its column, however narrow the face its words are set in | 12 |\n| two | `code` in a cell | 3 |\n| three | short | Omega |\n"
    );
    // The heads are eyebrows, in capitals.
    let (pitch, lines) = baselines("table", &markdown, "ALPHA", "Omega");
    let steps = steps(pitch, &lines);
    assert!(steps.len() >= 4, "too few lines: {steps:?}");
    assert!(
        steps.iter().all(|s| *s == 100 || *s == 150),
        "a line in a table steps one pitch or, past a hairline, one and a half: {steps:?}"
    );
}

#[test]
fn what_follows_a_table_stands_where_it_would_after_words() {
    let markdown = format!(
        "{OPENING}| Alpha | Beta |\n|---|---|\n| one | two |\n\n## Section\n\nWords.\n\n| Gamma | Delta |\n|---|---|\n| three | four |\n\nOmega.\n"
    );
    let (pitch, lines) = baselines("after-table", &markdown, "opening", "Omega");
    let steps = steps(pitch, &lines);
    // A table two lines from the words either side, its rows one and a
    // half apart past their hairline, and a section three lines under it
    // as under words.
    assert_eq!(
        steps[steps.len() - 7..],
        [200, 150, 300, 100, 200, 150, 200],
        "a table two lines off, a section three under it: {steps:?}"
    );
}

#[test]
fn a_records_lines_land_on_the_pitch() {
    let head = (1..=9)
        .map(|i| format!("Incomprehensibility{i}"))
        .collect::<Vec<_>>()
        .join(" | ");
    let rule = ["---"; 9].join(" | ");
    let row = |first: &str| {
        std::iter::once(first.to_string())
            .chain((2..=9).map(|i| format!("Counterrevolutionaries{i} and a value long enough to wrap onto a second line beside its header")))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let markdown = format!(
        "{OPENING}| {head} |\n| {rule} |\n| {} |\n| {} |\n",
        row("Alpha"),
        row("Omega")
    );
    let (pitch, lines) = baselines("records", &markdown, "Alpha", "Counterrevolutionaries9");
    let steps = steps(pitch, &lines);
    assert!(
        steps.len() >= 10,
        "set as records, too few lines: {steps:?}"
    );
    assert!(
        steps.iter().all(|s| *s == 100 || *s == 150),
        "a field or a wrap steps one pitch, a record one and a half: {steps:?}"
    );
}
