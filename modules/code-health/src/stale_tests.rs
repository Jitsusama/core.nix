//! The tests of the comments naming a path that's gone: what each language
//! reads as a comment, and where a path a comment names starts and ends.

use super::*;
use crate::measure::file;

fn paths(files: &[File]) -> Vec<String> {
    files.iter().map(|file| file.path.clone()).collect()
}

/// The comments of `files` naming what's gone, once every file's comments
/// were known, as each test but the reader's own misreadings needs.
fn stale(paths: &[String], files: &[File]) -> Vec<String> {
    let stale = super::stale(paths, files);
    assert_eq!(
        stale.unread,
        Vec::<String>::new(),
        "a file's comments couldn't be known"
    );
    stale.gone
}

#[path = "stale_shell_tests.rs"]
mod shell;

#[test]
fn a_file_whose_comments_cant_be_known_is_named_apart_from_what_is_gone() {
    let files = [
        file("modules/a/x.sh", "# Read by modules/a/gone.sh.\n"),
        file("modules/a/y.sh", "# Read by modules/a/gone.sh.\n"),
    ];
    let found = stale_by(&paths(&files), &files, |file| {
        if file.path.ends_with("x.sh") {
            None
        } else {
            comments(file)
        }
    });
    assert_eq!(
        (found.gone, found.unread),
        (
            vec!["modules/a/y.sh:1: modules/a/gone.sh".to_string()],
            vec!["modules/a/x.sh".to_string()]
        ),
        "a file whose comments can't be known wasn't named alone, apart from what's gone"
    );
}

#[test]
fn a_comment_naming_a_path_gone_is_stale() {
    let files = [
        file(
            "modules/a/home.nix",
            "# Read by modules/b/gone.py and modules/b.\n",
        ),
        file("modules/b/home.nix", "{ }\n"),
        file(
            "tools/c.rs",
            "// Moved from modules/b/c.rs, beside modules/a/home.nix.\n",
        ),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "modules/a/home.nix:1: modules/b/gone.py",
            "tools/c.rs:1: modules/b/c.rs"
        ],
        "a path gone was missed, or one there was taken for gone"
    );
}

#[test]
fn a_path_is_found_from_any_folder_above_its_file() {
    let files = [
        file(
            "modules/a/src/main.rs",
            "//! Its words are tests/kept/words.json.\n",
        ),
        file("modules/a/tests/kept/words.json", "{}\n"),
        file("tests/kept/a.nix", "{ }\n"),
    ];
    assert!(
        stale(&paths(&files), &files).is_empty(),
        "a path named from the file's own module was taken for gone"
    );
}

#[test]
fn only_comments_and_docstrings_are_read() {
    let files = [
        file(
            "tests/a.sh",
            "#!/usr/bin/env tests/kept/gone\nx=\"tests/kept/none.sh\"\n#! tests/kept/bang.sh\n",
        ),
        file(
            "tests/b.py",
            "\"\"\"Run by\ntests/kept/checks.nix.\n\"\"\"\nx = \"tests/kept/none.py\"\n",
        ),
        file("tests/c.md", "See tests/kept/none.\n"),
        file("tests/d.nix", "/* Beside\n * tests/kept/none.nix */\n{ }\n"),
        file(
            "tests/e.sh",
            "  #! tests/kept/lead.sh\n#! tests/kept/two.sh\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/a.sh:3: tests/kept/bang.sh",
            "tests/b.py:2: tests/kept/checks.nix",
            "tests/d.nix:2: tests/kept/none.nix",
            "tests/e.sh:1: tests/kept/lead.sh",
            "tests/e.sh:2: tests/kept/two.sh"
        ],
        "a string or a shebang was read, or a docstring, an indented `#!` or one past line 1 \
         missed"
    );
}

#[test]
fn block_comments_and_every_docstring_are_read_whole() {
    let files = [
        file(
            "tests/e.py",
            "r\"\"\"Raw, naming\ntests/kept/raw.nix.\n\"\"\"\ndef f():\n    \
             '''Says \"\"\" once\n    tests/kept/single.nix\n    '''\n    \
             return \"tests/kept/none.py\"\n",
        ),
        file(
            "modules/w/a.qml",
            "/**\n Beside\n tests/kept/jsdoc.qml\n */\n\
             Item { source: \"tests/kept/none.qml\" }\n",
        ),
        file(
            "tests/f.nix",
            "/*\n  Lines with no star,\n  tests/kept/bare.nix\n*/\n\
             { x = \"tests/kept/none.nix\"; }\n",
        ),
        file(
            "tests/h.py",
            "'''Opens, quoting \"\"\"\nand tests/kept/inside.nix\n'''\n\
             x = \"tests/kept/none.py\"\n",
        ),
        file(
            "tests/i.nix",
            "/* One line, naming tests/kept/one.nix */\n{ x = \"tests/kept/none.nix\"; }\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/e.py:2: tests/kept/raw.nix",
            "tests/e.py:6: tests/kept/single.nix",
            "modules/w/a.qml:3: tests/kept/jsdoc.qml",
            "tests/f.nix:3: tests/kept/bare.nix",
            "tests/h.py:2: tests/kept/inside.nix",
            "tests/i.nix:1: tests/kept/one.nix"
        ],
        "a raw docstring, one holding the other quotes, or a block's starless line was \
         missed, code past a docstring was read as one, or a one-line block stayed open"
    );
}

#[test]
fn an_opening_the_file_never_closes_hides_nothing() {
    let files = [
        file(
            "tests/o.py",
            "# Naming tests/kept/first.nix.\n\
             def f():\n\
             \x20   \"\"\"Closed, naming tests/kept/closed.nix.\"\"\"\n\
             def g(): \"\"\"Opens, naming tests/kept/doc.nix\n\
             \x20   and never closes\n\
             # tests/kept/past.nix, past it\n",
        ),
        file(
            "tests/p.nix",
            "# Naming tests/kept/top.nix.\n{ }\n/* Opens,\n   naming tests/kept/block.nix\n\
             # tests/kept/after.nix, past it\n",
        ),
        file(
            "tests/w.nix",
            "{ x = ''\n  /* in a string\n''; }\n# tests/kept/string.nix, past it\n",
        ),
        file(
            "tests/q.py",
            "# Naming tests/kept/before.nix.\nx = \"\"\"open\n# tests/kept/code.nix\n",
        ),
        file(
            "tests/u.py",
            "# Naming tests/kept/u.nix.\n\"\"\"Opens, naming tests/kept/open.nix\n\
             # tests/kept/beyond.nix, past it\n",
        ),
        file(
            "tests/v.py",
            "\"\"\"Opens, naming tests/kept/opens.nix\n\
             ends\"\"\" + '''never\n\
             # tests/kept/later.nix, past it\n",
        ),
        file(
            "tests/y.py",
            "x = \"\"\"\n\
             # tests/kept/inside.nix, in a string\n\
             \"\"\" + '''never\n\
             # tests/kept/after.nix, past it\n",
        ),
        file(
            "tests/z.py",
            "x = ('''never)\n\
             def f():\n\
             \x20   \"\"\"Naming tests/kept/bracketed.nix.\"\"\"\n",
        ),
        file(
            "tests/n.sh",
            "echo \"a\n\
             # tests/kept/in.sh, in a string\n\
             \" 'never\n\
             # tests/kept/closes.sh, past it\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/o.py:1: tests/kept/first.nix",
            "tests/o.py:3: tests/kept/closed.nix",
            "tests/o.py:6: tests/kept/past.nix",
            "tests/p.nix:1: tests/kept/top.nix",
            "tests/p.nix:5: tests/kept/after.nix",
            "tests/w.nix:4: tests/kept/string.nix",
            "tests/q.py:1: tests/kept/before.nix",
            "tests/q.py:3: tests/kept/code.nix",
            "tests/u.py:1: tests/kept/u.nix",
            "tests/u.py:3: tests/kept/beyond.nix",
            "tests/v.py:1: tests/kept/opens.nix",
            "tests/v.py:3: tests/kept/later.nix",
            "tests/y.py:4: tests/kept/after.nix",
            "tests/z.py:3: tests/kept/bracketed.nix",
            "tests/n.sh:4: tests/kept/closes.sh"
        ],
        "a docstring, a block comment, a string or a quote the file never closes, even one \
         opened where another closes, was read, or it hid what came before or after it, or \
         kept the bracket closing past it open"
    );
}

#[test]
fn each_opening_never_closed_is_read_past_in_turn() {
    let files = [
        file(
            "tests/t.py",
            "# Naming tests/kept/t.nix.\n\
             x = '''never\n\
             # tests/kept/first.py, past it\n\
             y = \"\"\"never\n\
             # tests/kept/second.py, past that\n",
        ),
        file(
            "tests/t.sh",
            "# Naming tests/kept/t.nix.\n\
             cat <<A\n\
             # tests/kept/first.sh, past it\n\
             echo \"never\n\
             # tests/kept/second.sh, past that\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/t.py:1: tests/kept/t.nix",
            "tests/t.py:3: tests/kept/first.py",
            "tests/t.py:5: tests/kept/second.py",
            "tests/t.sh:1: tests/kept/t.nix",
            "tests/t.sh:3: tests/kept/first.sh",
            "tests/t.sh:5: tests/kept/second.sh"
        ],
        "an opening never closed, found only once the one before it was read past, hid \
         what came after it"
    );
}

#[test]
fn a_docstring_stands_only_where_a_statement_starts() {
    let files = [
        file(
            "tests/g.py",
            "x = \"\"\"\nUsage, as tests/kept/none.py says:\n\"\"\"\ny = 1\ncall(\n    \
             f\"\"\"tests/kept/none.py\"\"\"\n)\n\n\ndef f():\n    \
             u\"\"\"One line, naming tests/kept/u.nix.\"\"\"\n    \
             return \"tests/kept/none.py\"\n\n\nclass K:\n    \
             Rb\"\"\"Opens here\n    and names tests/kept/rb.nix\n    \"\"\"\n    z = 2\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/g.py:11: tests/kept/u.nix",
            "tests/g.py:17: tests/kept/rb.nix"
        ],
        "a string's closing quotes or an argument were read as a docstring, a docstring \
         after a def or a class was missed, or a one-line docstring stayed open"
    );
}

#[test]
fn triple_quotes_in_a_string_or_a_comment_open_nothing() {
    let files = [
        file(
            "tests/j.py",
            "QUOTES = ('\"\"\"', \"'''\")\n\
             # tests/kept/string.nix, past a string's quotes\n\
             MARK = 1  # three quotes, ''', in a comment open nothing\n\
             # tests/kept/comment.nix, past a comment's quotes\n\
             def f(x=\"#\"):\n    \"\"\"Naming tests/kept/hash.nix.\"\"\"\n\
             s = 'it\\'s \"\"\" here'\n\
             # tests/kept/escape.nix, past an escaped quote\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/j.py:2: tests/kept/string.nix",
            "tests/j.py:4: tests/kept/comment.nix",
            "tests/j.py:6: tests/kept/hash.nix",
            "tests/j.py:8: tests/kept/escape.nix"
        ],
        "triple quotes inside a string or a comment, or past an escaped quote, were taken \
         for a string left open, or a def's `#` default hid its docstring"
    );
}

#[test]
fn quotes_are_read_past_a_comment_an_escape_and_a_strings_close() {
    let files = [
        file(
            "tests/k.py",
            "def g():  # a colon, then a comment\n\
             \x20   \"\"\"Naming tests/kept/colon.nix.\"\"\"\n\
             def h():\n    \"\"\"An escaped \\\"\"\" stays in, as tests/kept/quoted.nix says,\n\
             \x20   and tests/kept/after.nix too.\"\"\"\n\
             x = \"\"\"open\n\
             closed\"\"\" + \"\"\"again\n\
             # tests/kept/inside.nix, inside a string\n\
             \"\"\"\n\
             # tests/kept/out.nix, past it\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/k.py:2: tests/kept/colon.nix",
            "tests/k.py:4: tests/kept/quoted.nix",
            "tests/k.py:5: tests/kept/after.nix",
            "tests/k.py:10: tests/kept/out.nix"
        ],
        "a comment past a def's colon hid its docstring, an escaped quote closed one, or \
         a string opened past another's close was missed"
    );
}

#[test]
fn a_blank_line_a_comment_or_a_shebang_leaves_a_docstring_its_place() {
    let files = [
        file(
            "tests/n.py",
            "#!/usr/bin/env python3\n\
             \n\
             # A comment before the module's docstring.\n\
             \"\"\"Naming tests/kept/module.nix.\"\"\"\n\
             def f():\n\
             \x20   # A comment before its docstring.\n\
             \n\
             \x20   \n\
             \x20   \"\"\"Naming tests/kept/def.nix.\"\"\"\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/n.py:4: tests/kept/module.nix",
            "tests/n.py:9: tests/kept/def.nix"
        ],
        "a blank line, one of spaces alone, a comment or a shebang moved where a docstring \
         may stand"
    );
}

#[test]
fn strings_are_followed_from_where_each_line_truly_starts() {
    let files = [
        file(
            "tests/m.py",
            "def g():\n\
             \x20   \"\"\"Opened,\n\
             \x20   an escaped \\\"\"\" inside, then tests/kept/inner.nix,\n\
             \x20   and tests/kept/later.nix.\"\"\"\n\
             def h():\n\
             \x20   r\"\"\"A raw \\\"\"\" stays in, as tests/kept/raw.nix says,\n\
             \x20   and tests/kept/rawafter.nix.\"\"\"\n\
             def i():\n\
             \x20   \"\"\"Doc.\"\"\" if True else \"\"\"open\n\
             # tests/kept/inside.nix, in a string\n\
             \"\"\"\n\
             s = \"\"\"open\n\
             x\"\"\" + y  # a note:\n\
             \"\"\"Not a docstring, naming tests/kept/notdoc.nix.\"\"\"\n\
             t = 'one quote, \\\n\
             # tests/kept/cont.nix'\n\
             # tests/kept/out.nix, past them all\n\
             def j():\n\
             \x20   \"\"\"Opened,\n\
             \x20   closed\"\"\" + \"\"\"and a string opened\n\
             # tests/kept/handed.nix, in that string\n\
             \"\"\"\n\
             def k(x=\"\"\"a\n\
             b\"\"\"):\n\
             \x20   \"\"\"Naming tests/kept/colon.nix.\"\"\"\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/m.py:3: tests/kept/inner.nix",
            "tests/m.py:4: tests/kept/later.nix",
            "tests/m.py:6: tests/kept/raw.nix",
            "tests/m.py:7: tests/kept/rawafter.nix",
            "tests/m.py:17: tests/kept/out.nix",
            "tests/m.py:25: tests/kept/colon.nix"
        ],
        "an escape closed a docstring, a string opened past one's close was read as it, \
         a line past a string was read as if it began outside one, a backslash's \
         one-quote string was dropped, a string opened past a docstring's close was lost, \
         or a string closing on a colon hid the docstring after it"
    );
}

#[test]
fn a_docstring_stands_only_under_a_def_or_a_class() {
    let files = [
        file(
            "tests/h.py",
            "if ready:\n\
             \x20   \"\"\"An if's string, naming tests/kept/if.nix.\"\"\"\n\
             FIXTURES = {\n\
             \x20   \"key\":\n\
             \x20   \"\"\"A value, naming tests/kept/value.nix.\"\"\",\n\
             }\n\
             def f(\n\
             \x20   a,\n\
             \x20   b: int = (1, 2),\n\
             \x20   c: str = \"(\",\n\
             ) -> dict[str, int]:\n\
             \x20   \"\"\"Naming tests/kept/wrapped.nix.\"\"\"\n\
             class K(A, \\\n\
             \x20       B):\n\
             \x20   \"\"\"Naming tests/kept/carried.nix.\"\"\"\n\
             async  def g():\n\
             \x20   \"\"\"Naming tests/kept/async.nix.\"\"\"\n\
             def m(x) \\\n\
             \x20       -> int:\n\
             \x20   \"\"\"Naming tests/kept/backslash.nix.\"\"\"\n\
             if call(f(\n\
             \x20   x)):\n\
             \x20   \"\"\"A wrapped if's string, naming tests/kept/call.nix.\"\"\"\n\
             classes = {1:\n\
             \x20   \"\"\"Naming tests/kept/classes.nix.\"\"\"}\n\
             def h(): x = {\n\
             \x20   2:\n\
             \x20   \"\"\"A body's string, naming tests/kept/body.nix.\"\"\"}\n\
             async with lock:\n\
             \x20   \"\"\"An async with's string, naming tests/kept/with.nix.\"\"\"\n\
             async for x in y:\n\
             \x20   \"\"\"An async for's string, naming tests/kept/for.nix.\"\"\"\n\
             d = {\"\"\"a\n\
             b\"\"\":\n\
             \x20   \"\"\"A string past a key's, naming tests/kept/key.nix.\"\"\"}\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/h.py:12: tests/kept/wrapped.nix",
            "tests/h.py:15: tests/kept/carried.nix",
            "tests/h.py:17: tests/kept/async.nix",
            "tests/h.py:20: tests/kept/backslash.nix"
        ],
        "a string past an if's, a dict key's, a wrapped if's, a body's, an async with's or \
         for's, or a string key's colon was read as a docstring, or one past a header \
         carried across lines by brackets or a backslash, or an async def's, was missed"
    );
}

#[test]
fn a_docstring_may_stand_on_its_headers_own_line() {
    let files = [
        file(
            "tests/r.py",
            "class K: \"\"\"Naming tests/kept/class.nix.\"\"\"\n\
             def f(x: int = 1) -> dict[str, int]: '''Naming tests/kept/def.nix.'''\n\
             def g(x=\"tests/kept/none.py\"): \"\"\"Opens,\n\
             \x20   naming tests/kept/open.nix.\"\"\"\n\
             def h(key=lambda x: x): \"\"\"Naming tests/kept/lambda.nix.\"\"\"\n\
             def i(): return \"\"\"tests/kept/none.py\"\"\"\n\
             if x: \"\"\"tests/kept/none.py\"\"\"\n\
             def j(\n\
             \x20   a,\n\
             ): \"\"\"Naming tests/kept/wrapped.nix.\"\"\"\n\
             def k(x=\"\"\"a\n\
             b\"\"\"): \"\"\"Naming tests/kept/handed.nix.\"\"\"\n\
             def l(): d = {\n\
             }; f = lambda: \"\"\"tests/kept/none.py\"\"\"\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/r.py:1: tests/kept/class.nix",
            "tests/r.py:2: tests/kept/def.nix",
            "tests/r.py:4: tests/kept/open.nix",
            "tests/r.py:5: tests/kept/lambda.nix",
            "tests/r.py:10: tests/kept/wrapped.nix",
            "tests/r.py:12: tests/kept/handed.nix"
        ],
        "a docstring on its def's or class's own line was missed, a default was read, a \
         parameter's or a lambda's colon, or one past the header's, was taken for it, or a \
         return's or an if's string was read as a docstring"
    );
}

#[test]
fn a_bracket_closed_where_a_string_opens_still_closes() {
    let files = [
        file(
            "tests/s.py",
            "x = (1,\n\
             \x20    2) + \"\"\"a\n\
             b\"\"\"\n\
             def f():\n\
             \x20   \"\"\"Naming tests/kept/stuck.nix.\"\"\"\n\
             y = (1,\n\
             \x20    \"\"\"a\n\
             b\"\"\") + \"\"\"c\n\
             d\"\"\"\n\
             def g():\n\
             \x20   \"\"\"Naming tests/kept/inside.nix.\"\"\"\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/s.py:5: tests/kept/stuck.nix",
            "tests/s.py:11: tests/kept/inside.nix"
        ],
        "a bracket closed on a line leaving a string open, opened in code or past a \
         string's close, was lost, so the logical line never ended and the def's \
         docstring after it went unread"
    );
}

#[test]
fn a_bracket_in_a_strings_body_opens_nothing() {
    let files = [
        file(
            "tests/t.py",
            "x = \"\"\"(\n\
             [ {\n\
             \"\"\"\n\
             def f():\n\
             \x20   \"\"\"Naming tests/kept/after.nix.\"\"\"\n\
             y = '''a\n\
             )\n\
             '''\n\
             def g():\n\
             \x20   \"\"\"(\n\
             \x20   [\n\
             \x20   \"\"\"\n\
             def h():\n\
             \x20   \"\"\"Naming tests/kept/closer.nix.\"\"\"\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/t.py:5: tests/kept/after.nix",
            "tests/t.py:14: tests/kept/closer.nix"
        ],
        "a bracket inside a string or a docstring running across lines was counted, so a \
         def's docstring after it went unread"
    );
}

#[test]
fn a_byte_order_mark_hides_nothing_in_any_language() {
    let files = [
        file(
            "tests/b.py",
            "\u{feff}#!/usr/bin/env python3\n\"\"\"Naming tests/kept/shebang.nix.\"\"\"\n",
        ),
        file(
            "tests/d.py",
            "\u{feff}\"\"\"Naming tests/kept/doc.nix.\"\"\"\n",
        ),
        file("tests/r.rs", "\u{feff}// Naming tests/kept/rust.nix.\n"),
        file("tests/s.sh", "\u{feff}# Naming tests/kept/shell.nix.\n"),
        file(
            "tests/n.nix",
            "\u{feff}/* Naming tests/kept/block.nix. */\n{ }\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/b.py:2: tests/kept/shebang.nix",
            "tests/d.py:1: tests/kept/doc.nix",
            "tests/r.rs:1: tests/kept/rust.nix",
            "tests/s.sh:1: tests/kept/shell.nix",
            "tests/n.nix:1: tests/kept/block.nix"
        ],
        "a byte-order mark opening a file hid its shebang, its module's docstring or its \
         first comment"
    );
}

#[test]
fn a_path_below_no_folder_held_is_another_trees() {
    let files = [
        file(
            "tools/x/a.rs",
            "// As upstream's tools/token.c, and a stand-in's modules/one/a.\n",
        ),
        file("modules/b/home.nix", "{ }\n"),
        file(
            "refs/x/b.ts",
            "// Not refs/heads/main nor refs/remotes/origin.\n",
        ),
    ];
    assert!(
        stale(&paths(&files), &files).is_empty(),
        "another project's path, a stand-in's or a git ref was taken for gone"
    );
}

#[test]
fn a_path_cut_before_a_hyphen_names_where_names_begin() {
    let files = [
        file(
            "tools/a.rs",
            "// It owned only `tests/vm/screens-`, not tests/vm/gone-.\n\
             // Nor tests/vm/screens, which no file is.\n",
        ),
        file("tests/vm/gone.nix", "{ }\n"),
        file(
            "modules/b/src/c.rs",
            "// Beside its tests/kept/one- cases, not its tests/kept/two- ones.\n\
             // Read by tests/kept/x.nix- which is there.\n",
        ),
        file("modules/b/tests/kept/two.nix", "{ }\n"),
        file("modules/b/tests/kept/one-way.nix", "{ }\n"),
        file("tests/kept/x.nix", "{ }\n"),
        file("tests/vm/screens-come-vm-test.py", "\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tools/a.rs:1: tests/vm/gone",
            "tools/a.rs:2: tests/vm/screens",
            "modules/b/src/c.rs:1: tests/kept/two"
        ],
        "a beginning some path has was taken for gone, one only its letters begin, \
         or a whole name some path merely begins with, was missed"
    );
}

#[test]
fn a_path_is_cut_where_its_characters_end() {
    let files = [
        file(
            "tools/a.sh",
            "# See (tools/a.sh), tools/x/c.sh. and `tools/x/b`.\n",
        ),
        file(
            "tools/x/c.sh",
            "# Not my-tools/x/y, ./tools/x/y or $tools/x/y.\n",
        ),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        ["tools/a.sh:1: tools/x/b"],
        "a path was cut in the wrong place or named mid-word"
    );
}

#[test]
fn a_reading_naming_new_lines_is_read_again_until_it_reads_whole() {
    let mut asked = Vec::new();
    let lines = settled(|unopened| {
        let mut seen: Vec<usize> = unopened.iter().copied().collect();
        seen.sort_unstable();
        asked.push(seen);
        match asked.len() {
            1 => Err(vec![3]),
            2 => Err(vec![7]),
            _ => Ok(vec![(1, "# read")]),
        }
    });
    assert_eq!(
        (lines, asked),
        (Some(vec![(1, "# read")]), vec![vec![], vec![3], vec![3, 7]]),
        "a reading naming new lines wasn't read again with them unopened"
    );
}

#[test]
fn a_reading_naming_only_lines_named_before_leaves_the_comments_unknown() {
    let mut readings = 0;
    let lines = settled(|_| {
        readings += 1;
        Err(vec![4])
    });
    assert_eq!(
        (lines, readings),
        (None, 2),
        "a reading naming only a line named before was read again, or taken for whole"
    );
}
