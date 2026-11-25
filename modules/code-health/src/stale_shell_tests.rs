//! The tests of how shell is read: a heredoc's body, quotes and
//! substitutions as bash reads them, and where a `#` opens a comment.

use super::*;

#[test]
fn a_heredocs_body_is_no_comment() {
    let files = [
        file(
            "tests/h.sh",
            "#!/bin/sh\n\
             cat << EOF>out.sh\n\
             #!/usr/bin/env tests/kept/bang\n\
             # tests/kept/heredoc.sh, in a heredoc\n\
             EOF\n\
             # tests/kept/after.sh, past it\n\
             cat <<-'END'\n\
             \t# tests/kept/tabbed.sh, in a tabbed one\n\
             \tEND\n\
             # tests/kept/untabbed.sh, past that\n\
             cat <<\\STOP\n\
             # tests/kept/slashed.sh, in an escaped one\n\
             STOP\n\
             cat <<END-OF-HELP\n\
             # tests/kept/hyphen.sh, in a hyphenated one\n\
             END-OF-HELP\n\
             echo \"it's\" <<'my doc'\n\
             # tests/kept/spaced.sh, in one named with a space\n\
             my doc\n\
             cat <<A <<B\n\
             # tests/kept/a.sh, in the first of two\n\
             A\n\
             # tests/kept/b.sh, in the second\n\
             B\n\
             echo $((1 << 2)); cat <<C\n\
             # tests/kept/c.sh, in one past a shift\n\
             C\n\
             echo \\\"x <<F\n\
             # tests/kept/escaped.sh, in one past an escaped quote\n\
             F\n\
             echo \"\\\"\" <<G\n\
             # tests/kept/inquote.sh, in one past an escape in quotes\n\
             G\n\
             echo a#b <<K\n\
             # tests/kept/hash.sh, in one past a hash inside a word\n\
             K\n\
             cat <<L\n\
             # tests/kept/undashed.sh, in one a tabbed word closes not\n\
             \tL\n\
             # tests/kept/still.sh, in it still\n\
             L\n\
             cat <<E\"O\"F\n\
             # tests/kept/partly.sh, in one partly quoted\n\
             EOF\n\
             cat <<- M\n\
             # tests/kept/dashed.sh, in one dashed past a blank\n\
             \tM\n\
             # tests/kept/past.sh, past them all\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/h.sh:6: tests/kept/after.sh",
            "tests/h.sh:10: tests/kept/untabbed.sh",
            "tests/h.sh:48: tests/kept/past.sh"
        ],
        "a heredoc's body was read as comment, or one spaced, tabbed, escaped, hyphenated, \
         quoted, partly quoted, second on its line, past a shift, an escaped quote or a \
         hash inside a word, or its word before an operator, was missed, or a tabbed word \
         closed one without `<<-`"
    );
}

#[test]
fn only_a_heredocs_own_less_thans_open_one() {
    let files = [
        file(
            "tests/p.sh",
            "cat <<<x\n\
             # tests/kept/herestring.sh, past a here-string\n\
             x\n\
             echo hi # feeds <<D\n\
             # tests/kept/comment.sh, past a comment's\n\
             D\n\
             echo \"a <<E\"\n\
             # tests/kept/double.sh, past a double-quoted one\n\
             E\n\
             echo 'a <<H'\n\
             # tests/kept/single.sh, past a single-quoted one\n\
             H\n\
             echo $((x << y))\n\
             # tests/kept/shift.sh, past a shift\n\
             y\n\
             true;# feeds <<I\n\
             # tests/kept/semi.sh, past a comment's after an operator\n\
             I\n\
             echo \"a\n\
             <<J in a string\"\n\
             # tests/kept/string.sh, past a string's over lines\n\
             J\n\
             echo \"b\n\
             # tests/kept/inquote.sh, in a string\n\
             \"\n\
             echo \\'\n\
             # tests/kept/escaped.sh, past an escaped quote\n\
             echo \\'\n\
             cat <<NEVER\n\
             # tests/kept/rest.sh, past one never closed\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/p.sh:2: tests/kept/herestring.sh",
            "tests/p.sh:5: tests/kept/comment.sh",
            "tests/p.sh:8: tests/kept/double.sh",
            "tests/p.sh:11: tests/kept/single.sh",
            "tests/p.sh:14: tests/kept/shift.sh",
            "tests/p.sh:17: tests/kept/semi.sh",
            "tests/p.sh:21: tests/kept/string.sh",
            "tests/p.sh:27: tests/kept/escaped.sh",
            "tests/p.sh:30: tests/kept/rest.sh"
        ],
        "a here-string, a `<<` in a comment, even one past an operator, in quotes, even \
         over lines, or shifting in arithmetic opened a heredoc, a string's line was read as \
         comment, an escaped quote opened one, or a heredoc never closed hid the rest of its \
         file"
    );
}

#[test]
fn shell_quotes_are_read_as_bash_reads_them_in_substitutions_too() {
    let files = [
        file(
            "tests/b.sh",
            "echo $'it\\'s'\n\
             # tests/kept/ansi.sh, past a quote escaped in an ANSI one\n\
             x=\"$(echo \"a b\")\"\n\
             # tests/kept/nested.sh, past quotes in a substitution in quotes\n\
             y=\"${z:-\"a b\"}\"\n\
             # tests/kept/parameter.sh, past quotes in a parameter in quotes\n\
             w=\"`echo \"a\"`\"\n\
             # tests/kept/backtick.sh, past quotes in backticks in quotes\n\
             v=$(\n\
             \x20 # tests/kept/substituted.sh, a comment in a substitution\n\
             \x20 ( echo \"$(( 1 + 2 ))\" )\n\
             )\n\
             # tests/kept/closed.sh, past the substitution's close\n\
             u=\"${x#'#'}\" <<P\n\
             # tests/kept/inparameter.sh, in a heredoc past a parameter's hash\n\
             P\n\
             echo $(true)#x <<Q\n\
             # tests/kept/word.sh, in a heredoc past a hash in a substitution's word\n\
             Q\n\
             echo \"$(echo ')')\"\n\
             # tests/kept/paren.sh, past a bracket quoted in a substitution\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/b.sh:2: tests/kept/ansi.sh",
            "tests/b.sh:4: tests/kept/nested.sh",
            "tests/b.sh:6: tests/kept/parameter.sh",
            "tests/b.sh:8: tests/kept/backtick.sh",
            "tests/b.sh:10: tests/kept/substituted.sh",
            "tests/b.sh:13: tests/kept/closed.sh",
            "tests/b.sh:21: tests/kept/paren.sh"
        ],
        "a quote escaped in `$'...'`, quotes in a substitution or a parameter in quotes, or a \
         bracket of a substitution's own or in its quotes, was read wrong and hid a comment, \
         a comment in a substitution was missed, or a hash a parameter or a word holds kept \
         a heredoc from opening"
    );
}

// Each heredoc a misreading would open is closed after the comment it
// would hide, since one never closed is read again as opening nothing,
// which would hide the misreading too.
#[test]
fn a_hash_past_each_operator_opens_a_comment() {
    let files = [
        file(
            "tests/o.sh",
            "true|# <<A\n\
             # tests/kept/pipe.sh, past a pipe's\n\
             A\n\
             true&# <<B\n\
             # tests/kept/amp.sh, past an ampersand's\n\
             B\n\
             (# <<C\n\
             # tests/kept/open.sh, past an opening bracket's\n\
             C\n\
             true)# <<D\n\
             # tests/kept/close.sh, past a closing bracket's\n\
             D\n\
             true >#x <<E\n\
             # tests/kept/greater.sh, past a redirection's\n\
             E\n\
             cat <#x <<F\n\
             # tests/kept/less.sh, past an input's\n\
             F\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/o.sh:2: tests/kept/pipe.sh",
            "tests/o.sh:5: tests/kept/amp.sh",
            "tests/o.sh:8: tests/kept/open.sh",
            "tests/o.sh:11: tests/kept/close.sh",
            "tests/o.sh:14: tests/kept/greater.sh",
            "tests/o.sh:17: tests/kept/less.sh"
        ],
        "a `<<` in a comment past an operator opened a heredoc, hiding a comment after it"
    );
}

// Each case is a file of its own, so what one misreading leaves open can't
// be closed by another case's quotes. In each, the comment in the heredoc
// is hidden only when the line before is read right: one read wrong opens
// no heredoc, or leaves open what the file never closes, and is read again
// as opening nothing.
#[test]
fn what_a_line_leaves_open_is_what_bash_leaves_open() {
    let files = [
        file(
            "tests/ansi.sh",
            "echo $'it\\'s' <<A\n\
             # tests/kept/in-ansi.sh\n\
             A\n\
             # tests/kept/ansi.sh, past a quote an ANSI one closes\n",
        ),
        file(
            "tests/double.sh",
            "x=\"$(echo '\"')\" <<A\n\
             # tests/kept/in-double.sh\n\
             A\n\
             # tests/kept/double.sh, past a quote in a substitution in quotes\n",
        ),
        file(
            "tests/bracket.sh",
            "x=\"$( (echo) # )\"\n\
             )\" <<A\n\
             # tests/kept/in-bracket.sh\n\
             A\n\
             # tests/kept/bracket.sh, past a comment after a substitution's bracket\n",
        ),
        file(
            "tests/arithmetic.sh",
            "x=\"$( echo $((1)) # )\"\n\
             )\" <<A\n\
             # tests/kept/in-arithmetic.sh\n\
             A\n\
             # tests/kept/arithmetic.sh, past a comment after a substitution's sum\n",
        ),
        file(
            "tests/parameter.sh",
            "echo ${x:- #} <<A\n\
             # tests/kept/in-parameter.sh\n\
             A\n\
             # tests/kept/parameter.sh, past a parameter's hash past a blank\n",
        ),
        file(
            "tests/substitution.sh",
            "x=$(# <<B\n\
             # tests/kept/substituted.sh, a comment opening a substitution\n\
             )\n\
             B\n\
             case x in x) ;; esac\n\
             # tests/kept/substitution.sh, past it\n",
        ),
        file(
            "tests/named.sh",
            "cat <<A \"never closed\n\
             # tests/kept/named.sh, past a line read as opening nothing\n\
             A\n",
        ),
        // Each word stands on a later line, but the second's before the
        // first's, so the second heredoc's body runs to the file's end.
        file(
            "tests/order.sh",
            "#!/bin/sh\n\
             cat <<A <<B\n\
             B\n\
             A\n\
             # tests/kept/order.sh, past two heredocs closed out of turn\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/ansi.sh:4: tests/kept/ansi.sh",
            "tests/double.sh:4: tests/kept/double.sh",
            "tests/bracket.sh:5: tests/kept/bracket.sh",
            "tests/arithmetic.sh:5: tests/kept/arithmetic.sh",
            "tests/parameter.sh:4: tests/kept/parameter.sh",
            "tests/substitution.sh:2: tests/kept/substituted.sh",
            "tests/substitution.sh:6: tests/kept/substitution.sh",
            "tests/named.sh:2: tests/kept/named.sh",
            "tests/order.sh:5: tests/kept/order.sh"
        ],
        "an ANSI quote's close, a substitution in quotes, its brackets or its sum, a \
         parameter's hash, a comment opening a substitution, or a line read as opening \
         nothing was read other than as bash reads it, or a heredoc closed out of turn \
         was taken for another line's opening"
    );
}

// As above, each a file of its own, and each heredoc's comment hidden only
// when the line before is read as bash reads it.
#[test]
fn sums_cases_and_backticks_close_where_bash_closes_them() {
    let files = [
        file(
            "tests/sum.sh",
            "x=$(( $(echo 1) + 1 ))\n\
             cat <<A\n\
             # tests/kept/in-sum.sh\n\
             A\n\
             # tests/kept/sum.sh, past a substitution in a sum\n",
        ),
        file(
            "tests/brackets.sh",
            "x=$(( (1 + 2) * $(echo 3) )) <<A\n\
             # tests/kept/in-brackets.sh\n\
             A\n\
             # tests/kept/brackets.sh, past a sum's own brackets\n",
        ),
        file(
            "tests/case.sh",
            "x=\"$(case a in\n\
             a) echo hi ;;\n\
             esac\n\
             # tests/kept/case.sh, in a case in a substitution in quotes\n\
             )\" <<A\n\
             # tests/kept/in-case.sh\n\
             A\n",
        ),
        file(
            "tests/pattern.sh",
            "x=$(case a in (a) case b in b) echo ;; esac ;; esac) <<A\n\
             # tests/kept/in-pattern.sh\n\
             A\n\
             # tests/kept/pattern.sh, past cases in cases, a pattern bracketed\n",
        ),
        file(
            "tests/word.sh",
            "x=$(echo case x in y) <<A\n\
             # tests/kept/in-word.sh\n\
             A\n\
             # tests/kept/word.sh, past a case that is only a word\n",
        ),
        file(
            "tests/backtick.sh",
            "x=`echo a # it's` <<A\n\
             # tests/kept/in-backtick.sh\n\
             A\n\
             # tests/kept/backtick.sh, past a comment a backtick ends\n",
        ),
        // Bash opens a quote at an apostrophe in a default even in double
        // quotes, so the comment between is in it; a POSIX shell's isn't.
        file(
            "tests/default.sh",
            "echo \"${x:-it's}\"\n\
             # tests/kept/quoted.sh, in a quote a default opens\n\
             echo \"${x:-it's}\"\n\
             # tests/kept/default.sh, past it\n",
        ),
        file("tests/kept/x.nix", "{ }\n"),
    ];
    assert_eq!(
        stale(&paths(&files), &files),
        [
            "tests/sum.sh:5: tests/kept/sum.sh",
            "tests/brackets.sh:4: tests/kept/brackets.sh",
            "tests/case.sh:4: tests/kept/case.sh",
            "tests/pattern.sh:4: tests/kept/pattern.sh",
            "tests/word.sh:4: tests/kept/word.sh",
            "tests/backtick.sh:4: tests/kept/backtick.sh",
            "tests/default.sh:4: tests/kept/default.sh"
        ],
        "a substitution in a sum or a sum's own brackets, a case's patterns or a word \
         reading case, a comment in backticks, or an apostrophe in a quoted default was \
         read other than as bash reads it"
    );
}

/// What a line of shell, read from outside everything, leaves open.
fn left_open(line: &str) -> Vec<Shell> {
    let mut within = Vec::new();
    shell_line(line, &mut within, true);
    within
}

#[test]
fn each_line_leaves_open_what_bash_leaves_open() {
    use Shell::{Ansi, Arithmetic, Backtick, Case, Command, Double, Parameter, Single};
    let cases: [(&str, &[Shell]); 19] = [
        ("echo $'it\\'s", &[Ansi]),
        ("x=\"$(echo \"it's", &[Double, Command(0), Double]),
        ("y=\"${z:-\"it's", &[Double, Parameter, Double]),
        ("w=\"`echo \"it's", &[Double, Backtick, Double]),
        ("echo \"${x:-it's", &[Double, Parameter, Single]),
        ("x=$(( $(echo 1) + 1 ))", &[]),
        ("x=$(( (1 + 2", &[Arithmetic(1)]),
        ("(( x <<A", &[Arithmetic(0)]),
        ("echo ${x:-((}", &[]),
        ("x=$( (echo", &[Command(1)]),
        ("x=$( (#'", &[Command(1)]),
        ("x=$( (echo)#'", &[Command(0)]),
        ("x=$(case a in a)", &[Command(0), Case]),
        ("x=$(case a in a) ;; esac", &[Command(0)]),
        ("x=$(case a in a) echo esac)", &[Command(0), Case]),
        ("x=$(echo case a in a)", &[]),
        ("x=$(cased a in a)", &[]),
        ("x=`echo # it's`", &[]),
        ("x=`echo # it's", &[Backtick]),
    ];
    for (line, open) in cases {
        assert_eq!(
            left_open(line),
            open,
            "{line:?} left open other than bash does"
        );
    }
}

#[test]
fn heredocs_open_only_where_each_closes_in_turn() {
    // Each heredoc's word, with whether `<<-` lets tabs indent it.
    type Words = &'static [(&'static str, bool)];
    let closers = Closers::of("cat <<A <<B\nB\nA\nB\n\tC\n", true, false);
    let words = |words: &[(&str, bool)]| -> Vec<(String, bool)> {
        words
            .iter()
            .map(|&(word, tabbed)| (word.to_string(), tabbed))
            .collect()
    };
    let cases: [(Words, Words); 5] = [
        (&[("A", false), ("B", false)], &[("A", false), ("B", false)]),
        (&[("A", false), ("A", false)], &[("A", false)]),
        (
            &[("B", false), ("A", false), ("B", false)],
            &[("B", false), ("A", false), ("B", false)],
        ),
        (&[("C", false), ("A", false)], &[]),
        (&[("C", true)], &[("C", true)]),
    ];
    for (opened, closing) in cases {
        assert_eq!(
            closers.closing(words(opened), 0),
            words(closing),
            "heredocs {opened:?} on the first line closed other than in turn"
        );
    }
}

#[test]
fn a_line_is_named_for_what_it_leaves_open_not_what_it_closed() {
    let named = |content| {
        let closers = Closers::of(content, true, false);
        read(content, "sh", ("#", false), &closers, &HashSet::new())
    };
    assert_eq!(
        named("x=$(\n) y=$(\n"),
        Err(vec![1]),
        "a line closing one substitution and opening another wasn't named for the second"
    );
    assert_eq!(
        named("x=$(\n(echo\n"),
        Err(vec![0]),
        "a bracket counted in a substitution was taken for its opening"
    );
}

// Read again for each line holding one open, these would take seconds.
#[test]
fn what_never_closes_is_known_from_one_reading() {
    for opening in ["x=$(", "x=\"$(", "x=${"] {
        let content = format!("{opening}\n").repeat(3000) + "# tests/kept/end.sh\n";
        let closers = Closers::of(&content, true, false);
        let read = |unopened| read(&content, "sh", ("#", false), &closers, unopened);
        let none = HashSet::new();
        let openers = read(&none).expect_err("3000 openings never closed read as closed");
        assert_eq!(
            openers.len(),
            3000,
            "{opening:?} open on each line wasn't known from one reading"
        );
        assert!(
            read(&openers.into_iter().collect()).is_ok(),
            "{opening:?} read as opening nothing still left something open"
        );
    }
    let heredocs = "cat <<A <<A\n".repeat(3000) + "A\n";
    let blocks = "/*\n".repeat(3000);
    for (content, extension, comment) in [
        (&heredocs, "sh", ("#", false)),
        (&blocks, "nix", ("#", true)),
    ] {
        let closers = Closers::of(content, extension == "sh", comment.1);
        assert!(
            read(content, extension, comment, &closers, &HashSet::new()).is_ok(),
            "openings in {extension} no line closes in turn weren't known at once"
        );
    }
}
