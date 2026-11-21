//! How the page sets inline code: whole where it's short, broken only at
//! its spaces and after a slash where it's long, with no scrap left alone,
//! on the pitch, and in a table's column as the column measured it.

use super::super::pitch_pt;
use super::{OPENING, Setter, Width, baselines, steps};

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
    // One kept whole, and one long enough that the column measures its
    // widest held run and breaks it.
    for (i, code) in [
        "cargo build --release",
        "cargo build --release --features a-wide-column --target x86_64-unknown-linux-gnu",
    ]
    .into_iter()
    .enumerate()
    {
        code_in_a_narrow_column_keeps_to_it_when(&format!("cell-{i}"), code);
    }
}

/// Sets `code` in a column squeezed by a long cell beside it, in a
/// folder of its own named `name`, and checks it never reaches the next.
fn code_in_a_narrow_column_keeps_to_it_when(name: &str, code: &str) {
    let markdown = format!(
        "{OPENING}| Alpha | Command | Wraps |\n|---|---|---|\n| one | `{code}` | a cell long enough that it wraps onto a second line in its column, however narrow the face its words are set in, and longer still so it squeezes the others |\n"
    );
    let page = {
        let dir = std::env::temp_dir().join(format!("reader-{name}-{}", std::process::id()));
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
