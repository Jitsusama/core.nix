//! How a line of shell is read for its comments, as bash reads it: what
//! it leaves open, which heredocs it opens, and where each could close.

use std::collections::HashMap;

/// Where a line of shell stands, as bash reads it: in a quote, or in the
/// code of a substitution, inside which quotes open anew, as they do in
/// `"$(echo "a b")"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shell {
    /// `'...'`, which only its quote closes, a backslash being its own.
    Single,
    /// `$'...'`, where a backslash keeps a quote from closing it.
    Ansi,
    /// `"..."`, where a backslash escapes, and `$(`, `${` and a backtick
    /// open code again.
    Double,
    /// `$(...)`, code.
    Command,
    /// A bracket a command substitution's code opens, as `(cd x)`'s does,
    /// code whose `)` closes it rather than the substitution.
    Subshell,
    /// A backtick's command, code until the next backtick.
    Backtick,
    /// `${...}`, where quotes open as in code but a `#` is the
    /// parameter's, and `}` closes it. Bash opens a single quote here even
    /// inside double quotes, as `"${x:-it's}"` does, though a POSIX shell
    /// wouldn't.
    Parameter,
    /// `((...))` or `$((...))`, arithmetic, where substitutions open but
    /// `<<` shifts and `#` is a number's base.
    Arithmetic,
    /// A bracket arithmetic opens, as `(1 + 2)` does, arithmetic whose `)`
    /// closes it, so a `))` past it closes the sum no sooner.
    Grouping,
    /// A `case` in a command substitution, from its own word to its `in`.
    Case,
    /// A case's patterns, until the `)` ending them or the case's `esac`,
    /// where no reserved word but `esac` is one.
    Pattern,
    /// A pattern's commands, until the `;;`, `;&` or `;;&` ending them or
    /// the case's `esac`.
    Branch,
}

impl Shell {
    /// Whether what stands `within` this is code, where a line starting
    /// with `#` is a comment: outside every quote, or in a command
    /// substitution's lines.
    pub(super) fn code(within: Option<Shell>) -> bool {
        Self::runs(within) || matches!(within, Some(Shell::Case | Shell::Pattern))
    }

    /// Whether commands run where what stands `within` this is: outside
    /// every quote, or in a command substitution's commands or a case
    /// branch's, but not in a case's subject or its patterns.
    fn runs(within: Option<Shell>) -> bool {
        matches!(
            within,
            None | Some(Shell::Command | Shell::Subshell | Shell::Backtick | Shell::Branch)
        )
    }
}

/// The words closing the heredocs a line of shell opens, in the order
/// their bodies follow it, each with whether `<<-` lets tabs indent its
/// body, and the fewest things open at any point of the line, above which
/// stands what it leaves open of its own. `within` holds what lines
/// before left open, innermost last, which the line starts inside, and is
/// left holding what it leaves open. A line that `opens` nothing still
/// closes what it starts inside, but leaves open nothing of its own and
/// opens no heredoc. A `<<` opens a heredoc only in code, past no
/// comment, and outside arithmetic, where it shifts; a here-string's
/// `<<<` has no word after it, so it opens none.
pub(super) fn shell_line(
    line: &str,
    within: &mut Vec<Shell>,
    opens: bool,
) -> (Vec<(String, bool)>, usize) {
    let mut heredocs = Vec::new();
    let mut kept = within.len();
    // Whether a word begins here, as it does where a line or a command
    // substitution begins, or past a blank or an operator.
    let mut word_starts = true;
    // Whether a command may begin here: where a word does, but not past a
    // word and blanks, so `case` opens a case and `echo case` doesn't, nor
    // past a redirection, whose word is its target. In a case's patterns,
    // where a pattern list may begin. A line inside a case before its `in`
    // starts past the case's subject, since bash takes the subject only on
    // the case's own line and its `in` past any newlines.
    let mut command_starts = within.last() != Some(&Shell::Case);
    // Whether the character before was a redirection's, which `>|`, `>&`
    // and `<&` run on through.
    let mut redirected = false;
    let mut at = 0;
    while let Some(c) = line[at..].chars().next() {
        let rest = &line[at..];
        let mut step = c.len_utf8();
        let mut starts = false;
        // Whether no command or pattern list begins past this, though a
        // word may: past a redirection, or a pattern's `(` or `|`.
        let mut ends_none = false;
        let mut redirects = false;
        let innermost = within.last().copied();
        if let Some(counted) = bracket(innermost, rest, within) {
            (step, starts) = counted;
        } else if let Some(turned) =
            case_part(innermost, rest, (word_starts, command_starts), within)
        {
            (step, starts, ends_none) = turned;
        } else {
            match innermost {
                Some(Shell::Single | Shell::Ansi) if c == '\'' => {
                    within.pop();
                }
                Some(Shell::Ansi | Shell::Double) if c == '\\' => step = escaped(rest),
                Some(Shell::Single | Shell::Ansi) => {}
                Some(Shell::Double) if c == '"' => {
                    within.pop();
                }
                Some(Shell::Double) => {
                    if let Some(opened) = substitution(rest, within) {
                        (step, starts) = opened;
                    }
                }
                Some(Shell::Backtick) if c == '`' => {
                    within.pop();
                }
                // Bash finds a backtick's close before it reads the command,
                // so a comment in one ends there.
                Some(Shell::Backtick) if c == '#' && word_starts => {
                    step = rest.find('`').unwrap_or(rest.len());
                }
                Some(Shell::Parameter) if c == '}' => {
                    within.pop();
                }
                // Code: outside quotes, or in a substitution's.
                _ if c == '\\' => step = escaped(rest),
                _ if c == '\'' => within.push(Shell::Single),
                _ if rest.starts_with("$'") => {
                    within.push(Shell::Ansi);
                    step = 2;
                }
                _ if c == '"' => within.push(Shell::Double),
                // A `#` opens a comment only where a word begins, in code.
                _ if c == '#' && word_starts && Shell::code(innermost) => break,
                _ if rest.starts_with("((") && Shell::code(innermost) => {
                    within.push(Shell::Arithmetic);
                    step = 2;
                }
                _ if rest.starts_with("<<") && Shell::code(innermost) => {
                    step = 2;
                    if let Some((word, tabbed, length)) = heredoc_word(&rest[2..]) {
                        heredocs.push((word, tabbed));
                        step += length;
                    }
                }
                // A word begins past it, which may be the case's subject
                // alone, whatever it reads.
                _ if command_starts
                    && matches!(
                        innermost,
                        Some(Shell::Command | Shell::Subshell | Shell::Branch)
                    )
                    && reserved(rest, "case", false) =>
                {
                    within.push(Shell::Case);
                    (step, starts) = (4, true);
                }
                _ => {
                    if let Some(opened) = substitution(rest, within) {
                        (step, starts) = opened;
                    } else if let Some(length) =
                        leads_a_command(rest).filter(|_| command_starts && Shell::runs(innermost))
                    {
                        // A command may begin past a reserved word leading
                        // one, so a `case` past `then` opens a case.
                        (step, starts) = (length, true);
                    } else {
                        starts = c.is_whitespace() || ";&|()<>".contains(c);
                        redirects = "<>".contains(c) || (redirected && "|&".contains(c));
                        ends_none = redirects;
                    }
                }
            }
        }
        kept = kept.min(within.len());
        if !c.is_whitespace() {
            command_starts = starts && !ends_none;
        }
        redirected = redirects;
        word_starts = starts;
        at += step;
    }
    if !opens {
        within.truncate(kept);
        heredocs.clear();
    }
    (heredocs, kept)
}

/// The bracket `rest` opens with in what stands `innermost`, a command
/// substitution's code or arithmetic, pushed onto or popped from
/// `within`: how long it is, and whether a word begins past it. None for
/// any other text. Each bracket stands open as an entry of its own, so the
/// line that opened it is named with the rest should the file never close
/// it.
fn bracket(innermost: Option<Shell>, rest: &str, within: &mut Vec<Shell>) -> Option<(usize, bool)> {
    let (length, starts) = match innermost? {
        Shell::Command | Shell::Subshell | Shell::Branch
            if rest.starts_with('(') && !rest.starts_with("((") =>
        {
            within.push(Shell::Subshell);
            return Some((1, true));
        }
        // Past a substitution's close the word runs on; past a subshell's,
        // which is an operator, one begins. A sum's own bracket closes at
        // its `)`, a `))` past it closing the sum no sooner.
        Shell::Command | Shell::Grouping if rest.starts_with(')') => (1, false),
        Shell::Subshell if rest.starts_with(')') => (1, true),
        Shell::Arithmetic | Shell::Grouping if rest.starts_with('(') => {
            within.push(Shell::Grouping);
            return Some((1, false));
        }
        Shell::Arithmetic if rest.starts_with("))") => (2, false),
        _ => return None,
    };
    within.pop();
    Some((length, starts))
}

/// The turn of a case in a command substitution that `rest` opens with,
/// where `innermost` is a part of it, and `within` turned to match: how
/// long it is, whether a word begins past it, and whether neither a
/// command nor a pattern list does. None for any other text. A case's
/// subject is the first word past it, so only an `in` past a word ends
/// it, whatever the word reads. A pattern list begins past that `in`, past
/// the `;;`, `;&` or `;;&` ending a branch, and at a line's start, and an
/// `esac` there, or where a branch's command may begin, closes the case.
/// `starts` says whether a word, and a command or a pattern list, may
/// begin here.
fn case_part(
    innermost: Option<Shell>,
    rest: &str,
    (word_starts, command_starts): (bool, bool),
    within: &mut Vec<Shell>,
) -> Option<(usize, bool, bool)> {
    let c = rest.chars().next()?;
    let (turned, length) = match innermost? {
        Shell::Pattern | Shell::Branch if command_starts && reserved(rest, "esac", true) => {
            within.pop();
            return Some((4, false, false));
        }
        Shell::Case if word_starts && !command_starts && reserved(rest, "in", true) => {
            (Shell::Pattern, 2)
        }
        Shell::Pattern if c == ')' => (Shell::Branch, 1),
        // Another pattern stands past a pattern's own `(` or a `|`.
        Shell::Pattern if c == '(' || c == '|' => return Some((1, true, true)),
        // A `;;&` is a `;;` and an operator past it.
        Shell::Branch if rest.starts_with(";;") || rest.starts_with(";&") => (Shell::Pattern, 2),
        _ => return None,
    };
    within.pop();
    within.push(turned);
    Some((length, true, false))
}

/// How long the reserved word `rest` opens with is, where it's one past
/// which a command may begin: a keyword ending at a blank or an operator,
/// or `!` or `{` ending at a blank, as bash reads them.
fn leads_a_command(rest: &str) -> Option<usize> {
    const KEYWORDS: [&str; 8] = ["if", "then", "else", "elif", "do", "while", "until", "time"];
    KEYWORDS
        .iter()
        .find(|word| reserved(rest, word, true))
        .or_else(|| ["!", "{"].iter().find(|word| reserved(rest, word, false)))
        .map(|word| word.len())
}

/// Whether `rest` opens with the reserved `word`, ending at a blank or the
/// line's end, or at an operator too where `operator` says, as bash ends
/// one: `case=1` or `case.sh` is a plain word. A `case` an operator ends
/// is a pattern, as `case)` is, or no command bash runs, so only a blank
/// ends one.
fn reserved(rest: &str, word: &str, operator: bool) -> bool {
    rest.strip_prefix(word).is_some_and(|past| {
        past.chars()
            .next()
            .is_none_or(|c| c.is_whitespace() || (operator && ";&|()<>".contains(c)))
    })
}

/// The substitution `rest` opens, pushed onto `within`: how long its
/// opening is, and whether a word begins past it, as one does at a
/// command's start.
fn substitution(rest: &str, within: &mut Vec<Shell>) -> Option<(usize, bool)> {
    if rest.starts_with("$((") {
        within.push(Shell::Arithmetic);
        Some((3, false))
    } else if rest.starts_with("$(") {
        within.push(Shell::Command);
        Some((2, true))
    } else if rest.starts_with("${") {
        within.push(Shell::Parameter);
        Some((2, false))
    } else if rest.starts_with('`') {
        within.push(Shell::Backtick);
        Some((1, true))
    } else {
        None
    }
}

/// Where what a line opens could close, read once for a file: the lines
/// a heredoc's word might close, in order, by the line as it is and with
/// its leading tabs gone, as `<<-` lets them be, and the last line
/// holding a `*/`.
#[derive(Default)]
pub(super) struct Closers<'a> {
    whole: HashMap<&'a str, Vec<usize>>,
    untabbed: HashMap<&'a str, Vec<usize>>,
    block: Option<usize>,
}

impl<'a> Closers<'a> {
    /// The closers of `content`, its heredocs' for `shell` and its block
    /// comments' where it has `blocks`.
    pub(super) fn of(content: &'a str, shell: bool, blocks: bool) -> Closers<'a> {
        let mut closers = Closers::default();
        for (number, line) in content.lines().enumerate() {
            if shell {
                closers.whole.entry(line).or_default().push(number);
                closers
                    .untabbed
                    .entry(line.trim_start_matches('\t'))
                    .or_default()
                    .push(number);
            }
            if blocks && line.contains("*/") {
                closers.block = Some(number);
            }
        }
        closers
    }

    /// The heredocs of `words`, opened on line `number`, that close in
    /// turn, each body running on from the line closing the one before:
    /// from the first that no line closes, the line's `<<` was read wrong,
    /// and taken for nothing at once, the file not read again for it.
    pub(super) fn closing(&self, words: Vec<(String, bool)>, number: usize) -> Vec<(String, bool)> {
        let mut past = number;
        words
            .into_iter()
            .map_while(|heredoc| {
                let lines = if heredoc.1 {
                    &self.untabbed
                } else {
                    &self.whole
                };
                let lines = lines.get(heredoc.0.as_str())?;
                past = *lines.get(lines.partition_point(|&line| line <= past))?;
                Some(heredoc)
            })
            .collect()
    }

    /// Whether a line past `number` holds a `*/`, which could close a block
    /// comment it opens.
    pub(super) fn block_after(&self, number: usize) -> bool {
        self.block.is_some_and(|last| last > number)
    }
}

/// Whether `line` closes the heredoc whose body it's in: its word alone on
/// the line, past any tabs where `<<-` lets them indent it.
pub(super) fn ends_heredoc(line: &str, (word, tabbed): &(String, bool)) -> bool {
    let last = if *tabbed {
        line.trim_start_matches('\t')
    } else {
        line
    };
    last == word
}

/// How long a backslash opening `rest` is with the character it escapes.
fn escaped(rest: &str) -> usize {
    1 + rest[1..].chars().next().map_or(0, char::len_utf8)
}

/// The word a heredoc's `<<` is followed by in `rest`, read as the shell
/// reads it: past a `-`, which lets tabs indent the body, and any blanks,
/// up to a blank or an operator outside quotes, its quotes and backslashes
/// taken off as its closing line leaves them. With whether tabs may
/// indent, and how much of `rest` it took; none where no word follows.
fn heredoc_word(rest: &str) -> Option<(String, bool, usize)> {
    let (dashless, tabbed) = rest
        .strip_prefix('-')
        .map_or((rest, false), |rest| (rest, true));
    let mut at = rest.len() - dashless.trim_start_matches([' ', '\t']).len();
    let mut word = String::new();
    let mut quote = None;
    while let Some(c) = rest[at..].chars().next() {
        let mut step = c.len_utf8();
        match quote {
            Some(open) if c == open => quote = None,
            None if c == '\\' => {
                step = escaped(&rest[at..]);
                word.push_str(&rest[at + 1..at + step]);
            }
            None if c == '\'' || c == '"' => quote = Some(c),
            None if c.is_whitespace() || ";&|<>()".contains(c) => break,
            // Quoted or not, the rest is the word's.
            Some(_) | None => word.push(c),
        }
        at += step;
    }
    (!word.is_empty()).then_some((word, tabbed, at))
}
