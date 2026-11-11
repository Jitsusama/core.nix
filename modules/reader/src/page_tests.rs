use super::{Setter, Width};

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
    assert_eq!(width(1080.0), (435.0, spread), "the whole page");
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
        .map(|pair| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a step of a few pitches, rounded"
            )]
            let hundredths = ((pair[1] - pair[0]) / pitch * 100.0).round() as i64;
            hundredths
        })
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
fn a_page_ends_a_line_under_its_last_baseline() {
    let page = page_of("foot", &format!("{OPENING}Omega.\n"));
    let last = crate::text::runs(&page.frame)
        .iter()
        .map(|run| run.baseline_pt)
        .fold(f64::NEG_INFINITY, f64::max);
    let foot = page.frame.height().to_pt() - last;
    assert!(
        (foot - super::pitch_pt()).abs() < 0.01,
        "the page ends {foot} pt under its last baseline, not a line of {} pt",
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
