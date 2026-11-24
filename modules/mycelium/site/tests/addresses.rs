// The Rust copies' reader of the address cases (addresses.txt, beside this),
// one for both: the notices daemon's tests and mycelium-link's take it in
// with include!, so each holds its rule to the same cases read the same way.

/// The cases three copies of the address rule are held to, shared with the
/// map (README.md, The Address Cases).
const ADDRESSES: &str = include_str!("addresses.txt");

/// Hold `judged`, a copy of the address rule, to every case, failing on
/// the first it reads otherwise, or on a case written any way but one.
fn judged_by_every_case(judged: impl Fn(&str) -> bool) {
    let cases = cases_in(ADDRESSES);
    assert!(cases.len() > 20, "the shared cases are read");
    for (is, text) in cases {
        assert_eq!(judged(&text), is, "{text:?}");
    }
}

/// The cases `written` holds, each whether it is an address and its text,
/// failing on a line written any way but one, or on a carriage return
/// anywhere, a note's included (README.md, The Address Cases).
fn cases_in(written: &str) -> Vec<(bool, String)> {
    assert!(
        !written.contains('\r'),
        "{written:?} holds no carriage return"
    );
    // Split on line feeds alone, since `lines` would drop a carriage
    // return the other readers see.
    written
        .split('\n')
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (kind, text) = line.split_once(' ').unwrap_or((line, ""));
            assert!(matches!(kind, "is" | "not"), "{line:?} says is or not");
            (kind == "is", case_text(text))
        })
        .collect()
}

#[test]
fn a_line_written_any_other_way_is_refused() {
    for written in [
        "is T\r",
        "not T\r\n",
        "is T\rU",
        "# a note\r\nis T",
        "IS T",
        "maybe T",
    ] {
        let read = std::panic::catch_unwind(|| cases_in(written));
        assert!(read.is_err(), "{written:?} is refused");
    }
    assert_eq!(
        cases_in("# a note\nis T\n\nnot\nnot T "),
        [
            (true, "T".to_owned()),
            (false, String::new()),
            (false, "T ".to_owned())
        ],
        "a note and an empty line are skipped, and `not` alone is the empty text"
    );
}

#[test]
fn a_case_escaped_any_other_way_is_refused() {
    for written in [
        "T\\",
        "T\\n",
        "T\\u{a}",
        "T\\u{+A}",
        "T\\u{0000041}",
        "T\\u{}",
        "T\\u{41",
        "T\\u{D800}",
        "T\\u{DFFF}",
        "T\\u{110000}",
        "T\\\\",
    ] {
        let read = std::panic::catch_unwind(|| case_text(written));
        assert!(read.is_err(), "{written:?} is refused");
    }
    assert_eq!(
        case_text("T\\u{5C}\\u{9}"),
        "T\\\t",
        "each escape the one way"
    );
    assert_eq!(
        case_text("\\u{000041}\\u{10FFFF}\\u{E000}\\u{D7FF}"),
        "A\u{10FFFF}\u{E000}\u{D7FF}",
        "six digits, the top and each side of the surrogates are read"
    );
    assert_eq!(
        case_text("T\\u{5C}u{41}"),
        "T\\u{41}",
        "what an escape gives is never read again"
    );
}

/// A case's text with each `\u{HEX}`, 1 to 6 capital hex digits, made
/// the character it names, failing on any other backslash or a code that
/// names none (README.md, The Address Cases).
fn case_text(written: &str) -> String {
    let hex_digit = |digit: u8| digit.is_ascii_digit() || (b'A'..=b'F').contains(&digit);
    let mut text = String::new();
    let mut rest = written;
    while let Some(at) = rest.find('\\') {
        text.push_str(&rest[..at]);
        let escape = rest[at..]
            .strip_prefix("\\u{")
            .and_then(|after| after.split_once('}'))
            .filter(|(hex, _)| (1..=6).contains(&hex.len()) && hex.bytes().all(hex_digit));
        let (hex, after) = escape.unwrap_or_else(|| panic!("{written:?} escapes as \\u{{HEX}}"));
        let code = u32::from_str_radix(hex, 16).ok().and_then(char::from_u32);
        text.push(code.unwrap_or_else(|| panic!("{written:?} names a character")));
        rest = after;
    }
    text.push_str(rest);
    text
}
