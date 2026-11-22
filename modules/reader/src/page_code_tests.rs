//! How the page sets inline code: whole where it's short, broken only at
//! its spaces and after a slash where it's long, with no scrap left alone,
//! on the pitch, and in a table's column as the column measured it.

use super::super::pitch_pt;
use super::{OPENING, Setter, Width, baselines, steps};

/// How the page splits `code` into pieces on the measure a window
/// `window_pt` wide gets, as the template's own `code-pieces` says: each
/// piece after a `+` where it's held to the one before, or a `|` where a
/// line may break before it, and a `!` after one too wide to box, all on
/// one line.
fn held(code: &str, window_pt: f64) -> String {
    let theme = crate::theme::Theme::read("{}");
    let mut world = super::super::new_world();
    let (ink, look) = super::super::look(&theme, &world);
    // Set at a width no line of pieces reaches, so they read back as one.
    let program = format!(
        "{}#context text(size: 2pt, code-pieces({}).map(p => (if p.held {{ \"+\" }} else {{ \"|\" }}) + p.piece + (if p.whole {{ \"\" }} else {{ \"!\" }})).join(\" \"))",
        super::super::at_width(&ink, &look, Width::for_window(window_pt)),
        crate::walk::literal(code)
    );
    let page = super::super::set_page(&mut world, &program).expect("the page sets");
    crate::text::lines(crate::text::runs(&page.frame))
        .iter()
        .map(|line| line.text.trim_end().to_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The lines the page's template sets `body` as, given `s`, the code,
/// in a window `window_pt` wide.
fn probed(body: &str, code: &str, window_pt: f64) -> Vec<String> {
    let theme = crate::theme::Theme::read("{}");
    let mut world = super::super::new_world();
    let (ink, look) = super::super::look(&theme, &world);
    let program = format!(
        "{}#let s = {}\n#context {body}",
        super::super::at_width(&ink, &look, Width::for_window(window_pt)),
        crate::walk::literal(code)
    );
    let page = super::super::set_page(&mut world, &program).expect("the page sets");
    crate::text::lines(crate::text::runs(&page.frame))
        .iter()
        .map(|line| line.text.trim().to_owned())
        .collect()
}

#[test]
fn an_unboxed_piece_is_counted_in_the_stretches_typst_breaks_it_into() {
    let code = "x--y a-1-b --release f(a-) x-.y";
    // Its stretches as least-code reads them, and the lines Typst sets it
    // in when every break it may make is taken.
    let read = probed(
        r#"text(size: 2pt, s.matches(unbroken).map(m => m.text).filter(t => t != "").join("|"))"#,
        code,
        2560.0,
    )
    .join(" ");
    let broken = probed("box(width: 0.1pt, raw(s))", code, 2560.0).join("|");
    assert_eq!(
        read, broken,
        "least-code's stretches are where Typst breaks: after hyphens, \
         but not before a figure or a closing mark"
    );
    assert_eq!(read.matches('|').count(), 7, "it breaks at all: {read}");
}

#[test]
fn an_unboxed_pieces_first_stretch_counts_with_the_short_word_held_to_it() {
    // In a narrow window the name is unboxed, and "run" is held to it, so
    // the column needs "run a-" on one line, wider than any stretch alone.
    let widths = probed(
        r#"text(size: 2pt, (least-code(s), raw("www\u{a0}i-"), raw("i-"), raw("l-")).map(c => str(calc.round(measure(c).width.pt(), digits: 2))).join(" "))"#,
        &format!("www {}", ["i-l"; 120].join("-")),
        360.0,
    )
    .join(" ");
    let parsed: Vec<f64> = widths
        .split(' ')
        .map(|width| width.parse::<f64>().unwrap_or(f64::NAN))
        .collect();
    let [counted, joined, i, l] = parsed[..] else {
        panic!("four widths, not {widths:?}")
    };
    assert!(
        (counted - joined).abs() < 0.01 && joined > i.max(l),
        "least-code counts {counted} pt; www i- is {joined} pt, i- alone {i} pt, l- {l} pt"
    );
}

#[test]
fn short_pieces_hold_in_pairs_a_paths_opening_to_its_path_and_the_last_to_those_before() {
    assert_eq!(
        held("ls -l -a -h -t -r -S -R -1 -d -F -G -i", 300.0),
        "|ls +-l |-a +-h |-t +-r |-S +-R |-1 +-d |-F +-G +-i",
        "short flags pair, and the last joins the pair before it"
    );
    assert_eq!(
        held(
            "nix build --keep-going ~/.cache/pi/agentic | tee out",
            300.0
        ),
        "|nix +build |--keep-going +~/ +.cache/ |pi/ +agentic || +tee +out",
        "a path's opening holds to the rest of its path, and a pipe to what follows it"
    );
    assert_eq!(
        held(
            "cd modules/reader/src/page.rs && ./configure --prefix",
            300.0
        ),
        "|cd +modules/ |reader/ |src/ |page.rs +&& |./ +configure |--prefix",
        "a piece after a held pair stands free"
    );
    assert_eq!(
        held("git  log --oneline -n 20", 300.0),
        "|git + |log +--oneline |-n +20",
        "a doubled space is a piece of none, held to the word before it"
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

/// Code's pieces as the page splits them, at its spaces and after each
/// slash.
fn pieces(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for word in text.split(' ') {
        let mut piece = String::new();
        for c in word.chars() {
            piece.push(c);
            if c == '/' {
                found.push(std::mem::take(&mut piece));
            }
        }
        if !piece.is_empty() {
            found.push(piece);
        }
    }
    found
}

/// The short pieces of code broken over `lines` that share a line with
/// neither piece beside them.
fn scraps_alone(lines: &[&str]) -> Vec<String> {
    let placed: Vec<(usize, String)> = lines
        .iter()
        .enumerate()
        .flat_map(|(at, line)| pieces(line).into_iter().map(move |piece| (at, piece)))
        .collect();
    let on = |i: Option<usize>| i.and_then(|i| placed.get(i)).map(|(at, _)| *at);
    placed
        .iter()
        .enumerate()
        .filter(|(i, (at, piece))| {
            piece.chars().count() < 4
                && on(i.checked_sub(1)) != Some(*at)
                && on(Some(i + 1)) != Some(*at)
        })
        .map(|(_, (_, piece))| piece.clone())
        .collect()
}

#[test]
fn long_code_breaks_only_at_its_spaces_or_after_a_slash_and_never_leaves_a_scrap() {
    // Each piece narrower than two thirds of the narrowest measure here,
    // so each is kept whole; a wider one breaks where it can, as the next
    // test shows.
    let code = "nix-offload there --here-when-idle . checks.x86_64-linux.reader -L --keep-going ~/.cache/pi/agentic-harness.pi/work/trees/a-tree-with-a-long-name/tmp/runs/reader-sets-its-pages.log | tee out";
    let markdown = format!("Run this:\n\n`{code}`\n");
    let pitch = pitch_pt();
    let mut slashed = false;
    // Across windows from under 45 characters to past the whole page, so
    // the breaks land in many places, beside each scrap among them, and on
    // measures a wide window widens.
    let windows = (0..13).map(|i| 300.0 + 15.0 * f64::from(i));
    for window in windows.chain([560.0, 640.0, 720.0, 800.0, 1200.0]) {
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
        // A scrap under four characters keeps to a piece beside it, and a
        // path's opening to the rest of its path.
        let alone = scraps_alone(&texts);
        assert!(
            alone.is_empty(),
            "at {window} pt {alone:?} stand alone: {texts:?}"
        );
        assert!(
            !broken.contains("~/\n"),
            "at {window} pt a line ends on the path's opening: {texts:?}"
        );
        let baselines: Vec<f64> = lines.iter().map(|(_, baseline)| *baseline).collect();
        assert!(
            steps(pitch, &baselines).iter().all(|s| *s == 100),
            "at {window} pt the code's lines step otherwise: {baselines:?}"
        );
    }
    assert!(slashed, "no window broke the code after a slash");
}

#[test]
fn a_doubled_space_in_code_reads_back_as_written() {
    let code = "cargo test --package reader  --lib page::tests --no-fail-fast -- --nocapture";
    let lines = lines_from(
        "doubled",
        &format!("Run this:\n\n`{code}`\n"),
        "cargo",
        300.0,
    );
    let texts: Vec<&str> = lines.iter().map(|(text, _)| text.as_str()).collect();
    assert!(texts.len() >= 2, "the code took one line: {texts:?}");
    assert_eq!(texts.join(" "), code, "the code reads otherwise");
}

#[test]
fn a_span_of_short_flags_still_breaks_between_them() {
    // Every piece short: held to each other without end, they would run
    // off the page on one line.
    let flags = "ls -l -a -h -t -r -S -R -1 -d -F -G -i -k -n -o -p -q -s -u -x -X -Z -A -B -C";
    let lines = lines_from(
        "short-flags",
        &format!("Run this:\n\n`{flags}`\n"),
        "ls",
        300.0,
    );
    let texts: Vec<&str> = lines.iter().map(|(text, _)| text.as_str()).collect();
    assert!(texts.len() >= 2, "the flags took one line: {texts:?}");
    assert_eq!(texts.join(" "), flags, "the flags read otherwise");
    let alone = scraps_alone(&texts);
    assert!(alone.is_empty(), "{alone:?} stand alone: {texts:?}");
}

#[test]
fn a_piece_wider_than_two_thirds_of_the_measure_breaks_where_it_can() {
    let name = ["a-name"; 30].join("-");
    let lines = lines_from(
        "wide-piece",
        &format!("Run this:\n\n`{name}`\n"),
        "a-name",
        360.0,
    );
    assert!(lines.len() >= 2, "the name took {lines:?}");
    let read = lines
        .iter()
        .map(|(text, _)| text.as_str())
        .collect::<String>();
    assert_eq!(read, name, "the name reads otherwise");
    assert_eq!(
        held(&format!("run {name}"), 360.0),
        format!("|run +{name}!"),
        "a piece wider than two thirds of the measure is left unboxed"
    );
}

/// How a list with `name` in its fourth item sets: where the item starts
/// across, how wide the name sets, and where the furthest line ends.
fn set_in_a_list(name: &str, width: Width) -> (f64, f64, f64) {
    let markdown = format!("{OPENING}- One\n  - Two\n    - Three\n      - Four `{name}` here\n");
    let dir = std::env::temp_dir().join(format!(
        "reader-nested-{}-{}",
        std::process::id(),
        name.len()
    ));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(&file, &markdown).expect("the file");
    let mut setter = Setter::new(file.to_str().expect("a path"));
    setter.fit(width);
    let (page, _) = setter.set().expect("set");
    std::fs::remove_dir_all(&dir).expect("cleaned");
    let runs = crate::text::runs(&page.frame);
    let item = runs
        .iter()
        .find(|run| {
            run.chars
                .iter()
                .map(|c| c.0)
                .collect::<String>()
                .contains("Four")
        })
        .map(|run| run.x_pt)
        .expect("the item is set");
    let name_pt: f64 = runs
        .iter()
        .filter(|run| {
            let text: String = run.chars.iter().map(|c| c.0).collect();
            !text.trim().is_empty() && text.trim().chars().all(|c| "a-nme".contains(c))
        })
        .flat_map(|run| run.chars.iter().map(|c| c.2 - c.1))
        .sum();
    let right = runs
        .iter()
        .flat_map(|run| run.chars.iter().map(|c| c.2))
        .fold(0.0, f64::max);
    (item, name_pt, right)
}

#[test]
fn a_piece_wider_than_a_list_has_room_for_keeps_to_the_list() {
    // Over two thirds of a 360 pt window's measure, and under all of it,
    // in a list twice nested, which has less room than two thirds. The
    // name is sized from a first setting, as the gap between the list's
    // room and the measure is a few characters of whatever mono is set.
    let width = Width::for_window(360.0);
    let (item, probe_pt, _) = set_in_a_list(&"a-name".repeat(4), width);
    let room = width.measure_pt - item;
    let per_char = probe_pt / 24.0;
    let wanted = f64::midpoint(room, width.measure_pt) / per_char;
    let name: String = "a-name"
        .repeat(20)
        .chars()
        .take(crate::numbers::usize_of(wanted.round()))
        .collect();
    let (item, name_pt, right) = set_in_a_list(&name, width);
    let room = width.measure_pt - item;
    assert!(
        name_pt > room && name_pt < width.measure_pt,
        "the name is {name_pt} pt, not between the list's {room} pt and the measure"
    );
    assert_eq!(
        held(&format!("run {name}"), 360.0),
        format!("|run +{name}!"),
        "the name is left unboxed"
    );
    assert!(
        right <= width.measure_pt + 0.01,
        "a line runs to {right} pt, past the {} pt measure",
        width.measure_pt
    );
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
    // One kept whole; one long enough to break, whose widest run is the
    // triple's longest name held to the short pieces beside it; and a name
    // with nowhere to break, boxed in this wide a window, which the column
    // must hold whole.
    let hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    assert_eq!(
        held(&format!("sha256 {hash} here"), 2560.0),
        format!("|sha256 |{hash} |here"),
        "in the table's window the hash is boxed, so its column counts it whole"
    );
    assert_eq!(
        held(&format!("sha256 {hash} here"), 360.0),
        format!("|sha256 |{hash}! |here"),
        "in a narrow window the hash is unboxed, and counts its widest stretch"
    );
    for (i, code) in [
        "cargo build --release",
        "cargo build --release --features a-wide-column --target x86_64-unknown-linux-gnu -o out",
        "sha256 e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 here",
    ]
    .into_iter()
    .enumerate()
    {
        code_in_a_narrow_column_keeps_to_it_when(&format!("cell-{i}"), code, 2560.0);
    }
    // In a narrow window a long name is unboxed, so its column counts its
    // widest stretch between hyphens, the short word held to its first.
    let long = format!("run {}", ["a-name"; 30].join("-"));
    assert!(
        held(&long, 360.0).ends_with('!'),
        "the name is unboxed in a narrow window: {}",
        held(&long, 360.0)
    );
    code_in_a_narrow_column_keeps_to_it_when("cell-3", &long, 360.0);
}

/// Sets `code` in a column squeezed by a long cell beside it, in a window
/// `window_pt` wide and a folder of its own named `name`, and checks it
/// never reaches the next.
fn code_in_a_narrow_column_keeps_to_it_when(name: &str, code: &str, window_pt: f64) {
    let markdown = format!(
        "{OPENING}| Alpha | Command | Wraps |\n|---|---|---|\n| one | `{code}` | a cell long enough that it wraps onto a second line in its column, however narrow the face its words are set in, and longer still so it squeezes the others |\n"
    );
    let page = {
        let dir = std::env::temp_dir().join(format!("reader-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, &markdown).expect("the file");
        let mut setter = Setter::new(file.to_str().expect("a path"));
        setter.fit(Width::for_window(window_pt));
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
    let code_end = runs
        .iter()
        .filter(|run| {
            let text: String = run.chars.iter().map(|c| c.0).collect();
            text.trim().len() >= 3 && code.contains(text.trim())
        })
        .flat_map(|run| run.chars.iter())
        .map(|c| c.2)
        .fold(0.0, f64::max);
    assert!(code_end > 0.0, "{code:?} was never set");
    assert!(
        code_end < third,
        "{code:?} runs to {code_end} pt, past the next column at {third} pt"
    );
}

#[test]
fn short_code_is_kept_whole_on_one_line() {
    // After an opening, so no letter is dropped beside the lines.
    let markdown = "Short.\n\nKeep it short. ".to_owned()
        + &"Never run `cargo build` here, where it evaluates beside the work. ".repeat(12)
        + "\n";
    // Across several windows, so a break between its words is a choice
    // the lines would make somewhere were it not boxed.
    for window in [300.0, 330.0, 360.0, 390.0, 420.0, 450.0] {
        let lines = lines_from("whole", &markdown, "Keep it", window);
        assert!(lines.len() >= 6, "too few lines to show it: {lines:?}");
        let texts: Vec<&str> = lines.iter().map(|(text, _)| text.as_str()).collect();
        // Both its words are long enough to stand alone, so only the box
        // keeps them together.
        assert!(
            texts.iter().all(|text| !text.ends_with("cargo")),
            "at {window} pt a short span broke across lines: {texts:?}"
        );
    }
}
