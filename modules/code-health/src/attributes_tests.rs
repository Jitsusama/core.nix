//! The cases of the reader of Rust's attributes: each lint turned off
//! however its attribute is written, each line read past comments, strings
//! and characters as rustc reads them, and each line where a file is left
//! open, all read through the measure that counts them.

use crate::measure::{file, measure};

#[test]
fn a_rust_lint_is_counted_off_however_its_attribute_is_written() {
    // Each line opens with a quote, so this file's own lines aren't counted.
    let files = [file(
        "f.rs",
        concat!(
            "#[cfg_attr(test, allow(dead_code))]\n",
            "# [allow(unused)]\n",
            "#![cfg_attr(not(test), expect(clippy::x))]\n",
            "#[cfg_attr(test, derive(Debug))]\n",
            "# ! [ expect ( x ) ]\n",
            "#[cfg_attr(feature = \"x\", deny(y))]\n",
            "let s = \"#[cfg_attr(test, allow(x))]\";\n",
            "#[cfg_attr(test, disallow(x))]\n",
            "#[cfg_attr(test, derive(Debug), allow(x))]\n",
            "#[cfg_attr(a, cfg_attr(b, expect(x)))]\n",
            "#[allow_internal_unstable(x)]\n",
            "#[derive(Debug, allow(x))]\n",
            "#[cfg_attr(test, derive(Debug))] // a, allow(x) later\n",
            "#[cfg_attr(doc, doc = \"a, allow(x) here\")]\n",
            "#[cfg_attr(feature = \"a\\\", allow(b)\", deny(x))]\n",
            "#\u{a0}[allow(x)]\n",
            "#\u{200e}[allow(x)]\n",
            "#[cfg_attr(test, derive(Debug, allow(x)))]\n",
            "#[cfg_attr(\n",
            "    not(test), // a comment's ) and ]\n",
            "    expect(unused)\n",
            ")]\n",
            "\u{200e}\u{feff}\t#[allow(x)]\n",
            "#[derive(Debug)] #[allow(x)]\n",
            "#[derive(Debug)]\n",
            "#[allow(x)]\n",
            "#[cfg_attr(feature = r#\"a\")\"#, allow(x))]\n",
            "#[cfg_attr(feature = r#\"a\", allow(x)\"#, deny(x))]\n",
            "# /* a, allow(y) */ [allow(x)]\n",
            "#[cfg_attr(test, doc = ')', allow(x))]\n",
            "#[cfg_attr(test, doc = '\\\"', allow(x))]\n",
            "#[allow(x)\n",
        ),
    )];
    let health = measure(&["f.rs".to_owned()], &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        [
            "f.rs:1", "f.rs:2", "f.rs:3", "f.rs:5", "f.rs:9", "f.rs:10", "f.rs:17", "f.rs:19",
            "f.rs:23", "f.rs:24", "f.rs:26", "f.rs:27", "f.rs:29", "f.rs:30", "f.rs:31"
        ],
        "a lint turned off through `cfg_attr`, across lines, past another attribute or \
             with Rust's whitespace or a comment in its attribute was missed, or an \
             attribute turning none off, a comment, a string or a lint inside another \
             attribute was counted"
    );
}

#[test]
fn an_attribute_holding_brackets_closes_at_its_own() {
    let files = [file("g.rs", "#[my_attr[x]] #[allow(y)]\n")];
    let health = measure(&["g.rs".to_owned()], &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        ["g.rs:1"],
        "a bracket inside an attribute closed it, so the lint turned off past it was missed"
    );
}

#[test]
fn a_raw_string_after_a_letter_or_a_word_is_read_raw() {
    let files = [file(
        "r.rs",
        concat!(
            "#[cfg_attr(test, doc = br\"\\\", allow(x))]\n",
            "#[cfg_attr(test, doc = cr\"\\\", allow(x))]\n",
            "#[cfg_attr(test, path(a r\"\\\"), allow(x))]\n",
        ),
    )];
    let health = measure(&["r.rs".to_owned()], &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        ["r.rs:1", "r.rs:2", "r.rs:3"],
        "a byte or C raw string, or a raw string after a word, was read as a plain one, \
             so its closing quote was taken as escaped and the lint turned off past it missed"
    );
}

#[test]
fn a_raw_names_last_letter_opens_no_raw_string() {
    let files = [file(
        "n.rs",
        concat!(
            "m!(r#r\"\\\";\n#[allow(x)]\n\");\n#[allow(x)]\n",
            "m!('r#r\"\\\";\n#[allow(x)]\n\");\n#[allow(x)]\n",
            "#[cfg_attr(test, m(r#r\"\\\"]\"), allow(x))]\n",
        ),
    )];
    let health = measure(&["n.rs".to_owned()], &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        ["n.rs:4", "n.rs:8", "n.rs:9"],
        "a raw name or lifetime's last r was read as opening a raw string, so a string \
             after it was misread"
    );
}

#[test]
fn a_suffix_r_and_hash_make_no_raw_name() {
    let files = [file(
        "q.rs",
        concat!(
            "m!(1r#r\"\\\");\n#[allow(x)]\n// \");\n",
            "m!(1ur#r\"\\\");\n#[allow(x)]\n// \");\n",
            "m!(1_r#r\"\\\");\n#[allow(x)]\n// \");\n",
            "m!(1u\u{301}r#r\"\\\");\n#[allow(x)]\n// \");\n",
            "m!('a'r#r\"\\\");\n#[allow(x)]\n// \");\n",
        ),
    )];
    let health = measure(&["q.rs".to_owned()], &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        ["q.rs:2", "q.rs:5", "q.rs:8", "q.rs:11", "q.rs:14"],
        "a number's or character's suffix r and a # were read as a raw name"
    );
}

#[test]
fn a_numbers_suffix_r_in_an_attribute_opens_no_raw_string() {
    // Misread as raw, the string closes at the escaped quote and the real
    // closing quote opens one that runs to the end, hiding line 2.
    let files = [file(
        "o.rs",
        "#[cfg_attr(all(), cfg_attr(any(), m(1r\"\\\"),\")), allow(x))]\n#[allow(x)]\n",
    )];
    let health = measure(&["o.rs".to_owned()], &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        ["o.rs:1", "o.rs:2"],
        "a number's suffix r in an attribute was read as opening a raw string"
    );
}

#[test]
fn a_lint_named_rawly_is_counted_off() {
    let files = [file(
        "p.rs",
        concat!(
            "#[r#allow(x)]\n",
            "#[cfg_attr(test, r#expect(x))]\n",
            "#![r#allow(x)]\n",
        ),
    )];
    let health = measure(&["p.rs".to_owned()], &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        ["p.rs:1", "p.rs:2", "p.rs:3"],
        "a lint level named rawly, which rustc reads as the level itself, was missed"
    );
}

#[test]
fn a_suffix_opens_no_raw_string_and_rusts_whitespace_ends_a_name() {
    // Lines 2 and 6 sit in a string after a string's suffix ending r;
    // each third line from 10 to 28 follows a raw string after Rust's
    // whitespace past ASCII, and line 33 one closing at its second hash.
    let files = [file(
        "s.rs",
        concat!(
            "m!(\"a\"r\"\\\";\n#[allow(x)]\n\");\n#[allow(x)]\n",
            "m!(r\"a\"r\"\\\";\n#[allow(x)]\n\");\n#[allow(x)]\n",
            "m!(\u{85}r\"\\\");\n#[allow(x)]\n// \");\n",
            "m!(\u{200e}r\"\\\");\n#[allow(x)]\n// \");\n",
            "m!(\u{200f}r\"\\\");\n#[allow(x)]\n// \");\n",
            "m!(\u{2028}r\"\\\");\n#[allow(x)]\n// \");\n",
            "m!(\u{2029}r\"\\\");\n#[allow(x)]\n// \");\n",
            "m!(\u{2028}br\"\\\");\n#[allow(x)]\n// \");\n",
            "m!(r#x\u{2028}r\"\\\");\n#[allow(x)]\n// \");\n",
            "const R: &str = r##\"a \"# b\n#[allow(x)]\n\"##;\n#[allow(x)]\n",
        ),
    )];
    let health = measure(&["s.rs".to_owned()], &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        [
            "s.rs:4", "s.rs:8", "s.rs:10", "s.rs:13", "s.rs:16", "s.rs:19", "s.rs:22", "s.rs:25",
            "s.rs:28", "s.rs:33"
        ],
        "a string's suffix was read as opening a raw string, Rust's whitespace past \
             ASCII as part of a name, or a raw string as closed at fewer hashes than \
             opened it"
    );
}

#[test]
fn a_shebang_is_dropped_and_a_bang_opening_an_attribute_is_none() {
    let files = [
        file(
            "t.rs",
            "\u{feff}#!/x \"\n/* \"\n#[allow(x)]\n*/\n#[allow(x)]\n",
        ),
        file("u.rs", "#!/x \"\n#[allow(x)]\n// \"\n#[allow(x)]\n"),
        file("v.rs", "#! /* a */ [allow(x)]\n#[allow(x)]\n"),
        file("w.rs", "\u{feff}#![allow(x)]\n"),
    ];
    let paths = ["t.rs", "u.rs", "v.rs", "w.rs"].map(str::to_owned);
    let health = measure(&paths, &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        ["t.rs:5", "u.rs:2", "u.rs:4", "v.rs:1", "v.rs:2", "w.rs:1"],
        "a shebang's quote or comment was read as opening one, or a `#!` opening an \
             attribute, past a byte-order mark or not, was dropped as a shebang"
    );
}

#[test]
fn a_doc_comment_or_byte_order_mark_past_a_bang_makes_it_a_shebang() {
    let files = [
        file("a.rs", "#!/// a\n[allow(x)]\n"),
        file("b.rs", "#!//// a\n[allow(x)]\n"),
        file("c.rs", "#!//! a\n[allow(x)]\n"),
        file("d.rs", "#!/** */[ \"\n#[allow(x)]\n// \"\n"),
        file("e.rs", "#!/*** */[allow(x)]\n"),
        file("f.rs", "#!/**/[allow(x)]\n"),
        file("g.rs", "#!/*! */[allow(x)]\n"),
        file("h.rs", "#!\u{feff}[ \"\n#[allow(x)]\n// \"\n"),
    ];
    let paths = [
        "a.rs", "b.rs", "c.rs", "d.rs", "e.rs", "f.rs", "g.rs", "h.rs",
    ]
    .map(str::to_owned);
    let health = measure(&paths, &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        ["b.rs:1", "d.rs:2", "e.rs:1", "f.rs:1", "h.rs:2"],
        "a doc comment or byte-order mark after a `#!` was passed as rustc passes only \
             whitespace and plain comments there, or a plain comment was taken for a doc one"
    );
}

#[test]
fn only_a_line_opening_in_code_is_read() {
    // Each even line from 2 to 34 turns a lint off; those in code,
    // right after what would hide them from a walk misreading it, count.
    let files = [file(
        "k.rs",
        concat!(
            "/*\n#[allow(x)]\n*/\n#[allow(x)]\n",
            "const S: &str = \"\n#[allow(x)]\n\";\n#[allow(x)]\n",
            "const R: &str = r#\"\n#[allow(x)]\n\"#;\n#[allow(x)]\n",
            "/* /* */\n#[allow(x)]\n*/\n#[allow(x)]\n",
            "const Q: char = '\"'; const B: &[u8] = br\"\\\";\n#[allow(x)]\n",
            "m!('outer\"\\\"\n#[allow(x)]\n\");\n#[allow(x)]\n",
            "m!('r\"\\\"\n#[allow(x)]\n\");\n#[allow(x)]\n",
            "m!('br\"\\\"\n#[allow(x)]\n\");\n#[allow(x)]\n",
            "m!(xbr\"\\\"\n#[allow(x)]\n\");\n#[allow(x)]\n",
        ),
    )];
    let health = measure(&["k.rs".to_owned()], &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        [
            "k.rs:4", "k.rs:8", "k.rs:12", "k.rs:16", "k.rs:18", "k.rs:22", "k.rs:26", "k.rs:30",
            "k.rs:34"
        ],
        "a line opening inside a comment, string or raw string begun above was counted, \
             or one opening in code past a character, a byte raw string or a lifetime missed"
    );
}

#[test]
fn a_comment_or_string_left_open_at_the_end_counts_the_line_it_opens_on() {
    // a.rs, b.rs and c.rs leave a string, a nested comment and a raw
    // string open, each hiding a lint turned off below; d.rs opens one
    // on its last line, which no later line's walk passes; e.rs opens
    // one on a line counted already; f.rs leaves a comment open past a
    // shebang, where rustc reads on; and g.rs opens one below a line
    // counted, the last line walked.
    let files = [
        file("a.rs", "fn a() {}\nconst S: &str = \"\n#[allow(x)]\n"),
        file("b.rs", "/* /* */\n#[allow(x)]\n"),
        file("c.rs", "const R: &str = r#\"\n#[allow(x)]\n\"\n"),
        file("d.rs", "fn a() {}\nm!(\"\n"),
        file("e.rs", "#[allow(x)] /*\n"),
        file("f.rs", "#!/*\n#[allow(x)]\n"),
        file("g.rs", "#[allow(x)]\n/*\n"),
    ];
    let paths = ["a.rs", "b.rs", "c.rs", "d.rs", "e.rs", "f.rs", "g.rs"].map(str::to_owned);
    let health = measure(&paths, &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        [
            "a.rs:2", "b.rs:1", "c.rs:1", "d.rs:2", "e.rs:1", "f.rs:2", "g.rs:1", "g.rs:2"
        ],
        "a comment, string or raw string left open at the end didn't count the line it \
             opens on or counted it twice, or a comment left open past a shebang counted"
    );
}

#[test]
fn a_character_left_open_is_read_on_from_where_rustc_stops_it() {
    // rustc leaves one open at a line's end not followed by a quote (h.rs,
    // where the lifetime below gives a walk reading on a quote to close
    // at) or at a `/` (i.rs), and reads on from there; closes one across a
    // line's end a quote follows (j.rs); leaves one open at the file's end
    // (k.rs); and takes an escaped quote as escaped (l.rs), so the string
    // after it hides line 2. A quote before a symbol opens a character
    // (m.rs), not a lifetime before a string, which a quote before `_`,
    // a digit or a letter past ASCII opens (n.rs, o.rs and p.rs), hiding
    // line 2. q.rs leaves two open, on lines 2 and 4. In h.rs, i.rs and m.rs
    // the line one opens on is met after the line below it is counted.
    let files = [
        file("h.rs", "m!('\\n\n#[allow(x)]\nfn f<'a>() {}\n"),
        file("i.rs", "m!('\\x/ ');\n#[allow(x)]\n"),
        file("j.rs", "m!('\\x\n');\n#[allow(x)]\n"),
        file("k.rs", "m!('\\x"),
        file(
            "l.rs",
            "m!(['\\'','\"', \"\n#[allow(x)]\n\"]);\n#[allow(x)]\n",
        ),
        file("m.rs", "m!(' \"\n#[allow(x)]\n// \"\n"),
        file("n.rs", "m!('_ \"\n#[allow(x)]\n\");\n#[allow(x)]\n"),
        file("o.rs", "m!('0 \"\n#[allow(x)]\n\");\n#[allow(x)]\n"),
        file("p.rs", "m!('\u{e9} \"\n#[allow(x)]\n\");\n#[allow(x)]\n"),
        file("q.rs", "fn a() {}\nm!('\\x/);\n#[allow(x)]\nm!('\\x/);\n"),
    ];
    let paths = [
        "h.rs", "i.rs", "j.rs", "k.rs", "l.rs", "m.rs", "n.rs", "o.rs", "p.rs", "q.rs",
    ]
    .map(str::to_owned);
    let health = measure(&paths, &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        [
            "h.rs:1", "h.rs:2", "i.rs:1", "i.rs:2", "j.rs:3", "k.rs:1", "l.rs:4", "m.rs:1",
            "m.rs:2", "n.rs:4", "o.rs:4", "p.rs:4", "q.rs:2", "q.rs:3", "q.rs:4"
        ],
        "a character was closed where rustc leaves it open, left open where rustc closes \
             it, not read on from where rustc stops it, its line counted out of order, or \
             a lifetime was taken for one"
    );
}

#[test]
fn a_byte_character_or_a_quote_after_a_name_is_read_as_rustc_reads_it() {
    // A byte character's quote opens no lifetime, so r.rs's is left open
    // at its line's end, while s.rs's `xb` is a name before a lifetime,
    // whose string hides line 2, and y.rs's raw name `r#b`, like ys.rs's
    // string suffix and yc.rs's character suffix `b`, ends before a
    // lifetime and a character, so line 2 is code. A quote after the
    // name a lifetime would open closes a character (t.rs), but not one
    // further on (u.rs, or z.rs, where a quote follows), so each string
    // after hides line 2. v.rs's emoji opens a lifetime, as in rustc,
    // and so here does w.rs's arrow, which rustc reads as a character.
    // x.rs's walk reads on from the `/` where its character stops, not
    // past it, so the comment it opens hides the quote after.
    let files = [
        file("r.rs", "m!(b'a \"\n#[allow(x)]\n// \"\n"),
        file("s.rs", "m!(xb'a \"\n#[allow(x)]\n// \"\n#[allow(x)]\n"),
        file("t.rs", "m!('ab' \"\n#[allow(x)]\n// \"\n#[allow(x)]\n"),
        file("u.rs", "m!('a\"\n#[allow(x)]\n\");\n#[allow(x)]\n"),
        file("v.rs", "m!('\u{1f600} \"\n#[allow(x)]\n\");\n#[allow(x)]\n"),
        file("w.rs", "m!('\u{2192} \"\n#[allow(x)]\n\");\n#[allow(x)]\n"),
        file("x.rs", "m!('\\x// \"\n#[allow(x)]\n// \"\n"),
        file("y.rs", "m!(r#b'a '\"');\n#[allow(x)]\nfn f() {} // \"\n"),
        file(
            "ys.rs",
            "m!(\"x\"b'a '\"');\n#[allow(x)]\nfn f() {} // \"\n",
        ),
        file("yc.rs", "m!('x'b'a '\"');\n#[allow(x)]\nfn f() {} // \"\n"),
        file("z.rs", "m!('a \"'\n#[allow(x)]\n\");\n#[allow(x)]\n"),
    ];
    let paths = [
        "r.rs", "s.rs", "t.rs", "u.rs", "v.rs", "w.rs", "x.rs", "y.rs", "ys.rs", "yc.rs", "z.rs",
    ]
    .map(str::to_owned);
    let health = measure(&paths, &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        [
            "r.rs:1", "r.rs:2", "s.rs:4", "t.rs:4", "u.rs:4", "v.rs:4", "w.rs:4", "x.rs:1",
            "x.rs:2", "y.rs:2", "ys.rs:2", "yc.rs:2", "z.rs:4"
        ],
        "a byte character, a raw name's or suffix's `b`, a character closing after a name, a \
             lifetime before a quote or past ASCII, or a character stopping at a comment \
             was read otherwise"
    );
}

#[test]
fn a_cfg_attr_splits_only_at_commas_outside_its_brackets() {
    let files = [file(
        "h.rs",
        concat!(
            "#[cfg_attr(test, doc[a, allow(x)])]\n",
            "#[cfg_attr(test, doc{a, allow(x)})]\n",
            "#[cfg_attr(test, doc[a], allow(y))]\n",
            "#[cfg_attr(test, doc{a}, allow(y))]\n",
            "#[cfg_attr(test, a}, allow(y))]\n",
        ),
    )];
    let health = measure(&["h.rs".to_owned()], &files, &[]);
    assert_eq!(
        health.named["suppressions"],
        ["h.rs:3", "h.rs:4", "h.rs:5"],
        "a cfg_attr was split inside an attribute's brackets, or not read on past their \
             close, or a close it never opened stopped its reading"
    );
}
