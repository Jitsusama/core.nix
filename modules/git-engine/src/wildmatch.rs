//! git's glob matching, as wildmatch.c does it: `*` stops at a slash when
//! matching paths, `**` crosses them, `?` and character classes match one
//! character. Ported rule for rule, since ignore files and attributes have
//! to mean exactly what they mean to git.

#[derive(PartialEq, Eq, Clone, Copy)]
enum Outcome {
    Match,
    NoMatch,
    AbortAll,
    AbortToStarStar,
}

/// Whether `text` matches `pattern`. With `pathname`, wildcards other than
/// `**` don't match a slash.
pub fn matches(pattern: &[u8], text: &[u8], pathname: bool, fold: bool) -> bool {
    dowild(pattern, 0, text, 0, pathname, fold) == Outcome::Match
}

fn is_glob_special(c: u8) -> bool {
    matches!(c, b'*' | b'?' | b'[' | b'\\')
}

fn at(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

fn lower(c: u8, fold: bool) -> u8 {
    if fold { c.to_ascii_lowercase() } else { c }
}

#[allow(clippy::too_many_lines)]
fn dowild(p: &[u8], mut pi: usize, t: &[u8], mut ti: usize, pathname: bool, fold: bool) -> Outcome {
    let start = pi;
    while at(p, pi) != 0 {
        let mut p_ch = lower(at(p, pi), fold);
        let mut t_ch = lower(at(t, ti), fold);
        if t_ch == 0 && p_ch != b'*' {
            return Outcome::AbortAll;
        }
        match p_ch {
            b'?' => {
                if pathname && t_ch == b'/' {
                    return Outcome::NoMatch;
                }
            }
            b'*' => {
                let match_slash;
                pi += 1;
                if at(p, pi) == b'*' {
                    let prev = pi.checked_sub(2);
                    while at(p, pi + 1) == b'*' {
                        pi += 1;
                    }
                    pi += 1;
                    let after_slash = prev.is_none_or(|prev| prev < start || p[prev] == b'/');
                    let next = at(p, pi);
                    if !pathname {
                        match_slash = true;
                    } else if after_slash
                        && (next == 0 || next == b'/' || (next == b'\\' && at(p, pi + 1) == b'/'))
                    {
                        if next == b'/'
                            && dowild(p, pi + 1, t, ti, pathname, fold) == Outcome::Match
                        {
                            return Outcome::Match;
                        }
                        match_slash = true;
                    } else {
                        match_slash = false;
                    }
                } else {
                    match_slash = !pathname;
                }
                if at(p, pi) == 0 {
                    if !match_slash && t[ti..].contains(&b'/') {
                        return Outcome::NoMatch;
                    }
                    return Outcome::Match;
                } else if !match_slash && at(p, pi) == b'/' {
                    let Some(slash) = t[ti..].iter().position(|&c| c == b'/') else {
                        return Outcome::NoMatch;
                    };
                    // The slash is matched by the step at the end of the loop.
                    ti += slash;
                    pi += 1;
                    ti += 1;
                    continue;
                }
                loop {
                    if t_ch == 0 {
                        break;
                    }
                    let next = at(p, pi);
                    if !is_glob_special(next) {
                        let want = lower(next, fold);
                        loop {
                            t_ch = lower(at(t, ti), fold);
                            if t_ch == 0 || (!match_slash && t_ch == b'/') || t_ch == want {
                                break;
                            }
                            ti += 1;
                        }
                        if t_ch != want {
                            return if match_slash {
                                Outcome::AbortAll
                            } else {
                                Outcome::AbortToStarStar
                            };
                        }
                    }
                    let matched = dowild(p, pi, t, ti, pathname, fold);
                    if matched != Outcome::NoMatch {
                        if !match_slash || matched != Outcome::AbortToStarStar {
                            return matched;
                        }
                    } else if !match_slash && t_ch == b'/' {
                        return Outcome::AbortToStarStar;
                    }
                    ti += 1;
                    t_ch = lower(at(t, ti), fold);
                }
                return Outcome::AbortAll;
            }
            b'[' => {
                pi += 1;
                p_ch = at(p, pi);
                let negated = p_ch == b'!' || p_ch == b'^';
                if negated {
                    pi += 1;
                    p_ch = at(p, pi);
                }
                let mut prev_ch = 0u8;
                let mut matched = false;
                loop {
                    if p_ch == 0 {
                        return Outcome::AbortAll;
                    }
                    if p_ch == b'\\' {
                        pi += 1;
                        p_ch = at(p, pi);
                        if p_ch == 0 {
                            return Outcome::AbortAll;
                        }
                        if t_ch == p_ch {
                            matched = true;
                        }
                    } else if p_ch == b'-'
                        && prev_ch != 0
                        && at(p, pi + 1) != 0
                        && at(p, pi + 1) != b']'
                    {
                        pi += 1;
                        p_ch = at(p, pi);
                        if p_ch == b'\\' {
                            pi += 1;
                            p_ch = at(p, pi);
                            if p_ch == 0 {
                                return Outcome::AbortAll;
                            }
                        }
                        if t_ch <= p_ch && t_ch >= prev_ch {
                            matched = true;
                        } else if fold && t_ch.is_ascii_lowercase() {
                            let upper = t_ch.to_ascii_uppercase();
                            if upper <= p_ch && upper >= prev_ch {
                                matched = true;
                            }
                        }
                        p_ch = 0;
                    } else if p_ch == b'[' && at(p, pi + 1) == b':' {
                        pi += 2;
                        let s = pi;
                        while at(p, pi) != 0 && at(p, pi) != b']' {
                            pi += 1;
                        }
                        if at(p, pi) == 0 {
                            return Outcome::AbortAll;
                        }
                        if pi == s || p[pi - 1] != b':' {
                            // No ":]", so a plain '[' in the set.
                            pi = s - 2;
                            p_ch = b'[';
                            if t_ch == p_ch {
                                matched = true;
                            }
                        } else {
                            let class = &p[s..pi - 1];
                            let hit = match class {
                                b"alnum" => t_ch.is_ascii_alphanumeric(),
                                b"alpha" => t_ch.is_ascii_alphabetic(),
                                b"blank" => t_ch == b' ' || t_ch == b'\t',
                                b"cntrl" => t_ch.is_ascii_control(),
                                b"digit" => t_ch.is_ascii_digit(),
                                b"graph" => t_ch.is_ascii_graphic(),
                                b"lower" => t_ch.is_ascii_lowercase(),
                                b"print" => t_ch.is_ascii_graphic() || t_ch == b' ',
                                b"punct" => t_ch.is_ascii_punctuation(),
                                b"space" => t_ch.is_ascii_whitespace() || t_ch == 0x0b,
                                b"upper" => {
                                    t_ch.is_ascii_uppercase() || (fold && t_ch.is_ascii_lowercase())
                                }
                                b"xdigit" => t_ch.is_ascii_hexdigit(),
                                _ => return Outcome::AbortAll,
                            };
                            if hit {
                                matched = true;
                            }
                            p_ch = 0;
                        }
                    } else if t_ch == p_ch {
                        matched = true;
                    }
                    prev_ch = p_ch;
                    pi += 1;
                    p_ch = at(p, pi);
                    if p_ch == b']' {
                        break;
                    }
                }
                if matched == negated || (pathname && t_ch == b'/') {
                    return Outcome::NoMatch;
                }
            }
            _ => {
                if p_ch == b'\\' {
                    pi += 1;
                    p_ch = lower(at(p, pi), fold);
                }
                if t_ch != p_ch {
                    return Outcome::NoMatch;
                }
            }
        }
        pi += 1;
        ti += 1;
    }
    if ti < t.len() {
        Outcome::NoMatch
    } else {
        Outcome::Match
    }
}

#[cfg(test)]
mod tests {
    use super::matches;

    fn path(p: &str, t: &str) -> bool {
        matches(p.as_bytes(), t.as_bytes(), true, false)
    }

    fn plain(p: &str, t: &str) -> bool {
        matches(p.as_bytes(), t.as_bytes(), false, false)
    }

    #[test]
    fn without_paths_a_star_crosses_slashes() {
        assert!(plain("foo*bar", "foo/baz/bar"));
        assert!(plain("*/foo", "bar/baz/foo"));
        assert!(!path("*/foo", "bar/baz/foo"));
    }

    // Cases from git's t/t3070-wildmatch.sh, as (pattern, text, whether
    // they match as paths); the last field is unused.
    #[test]
    fn agrees_with_gits_own_cases() {
        let cases: &[(&str, &str, bool, bool)] = &[
            ("foo", "foo", true, true),
            ("bar", "foo", false, false),
            ("", "", true, true),
            ("???", "foo", true, true),
            ("??", "foo", false, false),
            ("*", "foo", true, true),
            ("f*", "foo", true, true),
            ("*f", "foo", false, false),
            ("*foo*", "foo", true, true),
            ("*ob*a*r*", "foobar", true, true),
            ("*ab", "aaaaaaabababab", true, true),
            ("foo\\*", "foo*", true, true),
            ("foo\\*bar", "foobar", false, false),
            ("f\\\\oo", "f\\oo", true, true),
            ("*[al]?", "ball", true, true),
            ("[ten]", "ten", false, false),
            ("**[!te]", "ten", true, true),
            ("**[!ten]", "ten", false, false),
            ("t[a-g]n", "ten", true, true),
            ("t[!a-g]n", "ten", false, false),
            ("t[!a-g]n", "ton", true, true),
            ("t[^a-g]n", "ton", true, true),
            ("a[]]b", "a]b", true, true),
            ("a[]-]b", "a-b", true, true),
            ("a[]-]b", "a]b", true, true),
            ("a[]-]b", "aab", false, false),
            ("a[]a-]b", "aab", true, true),
            ("]", "]", true, true),
            ("foo*bar", "foo/baz/bar", false, true),
            ("foo**bar", "foo/baz/bar", false, true),
            ("foo**bar", "foobazbar", true, true),
            ("foo/**/bar", "foo/baz/bar", true, true),
            ("foo/**/**/bar", "foo/baz/bar", true, true),
            ("foo/**/bar", "foo/b/a/z/bar", true, true),
            ("foo/**/**/bar", "foo/b/a/z/bar", true, true),
            ("foo/**/bar", "foo/bar", true, true),
            ("foo/**/**/bar", "foo/bar", true, true),
            ("foo?bar", "foo/bar", false, true),
            ("foo[/]bar", "foo/bar", false, true),
            ("foo[^a-z]bar", "foo/bar", false, true),
            ("f[^eiu][^eiu][^eiu][^eiu][^eiu]r", "foo/bar", false, true),
            ("f[^eiu][^eiu][^eiu][^eiu][^eiu]r", "foo-bar", true, true),
            ("**/foo", "foo", true, true),
            ("**/foo", "XXX/foo", true, true),
            ("**/foo", "bar/baz/foo", true, true),
            ("*/foo", "bar/baz/foo", false, true),
            ("**/bar*", "foo/bar/baz", false, false),
            ("**/bar/*", "deep/foo/bar/baz", true, true),
            ("**/bar/*", "deep/foo/bar/baz/", false, true),
            ("**/bar/**", "deep/foo/bar/baz/", true, true),
            ("**/bar/*", "deep/foo/bar", false, false),
            ("**/bar/**", "deep/foo/bar/", true, true),
            ("**/bar**", "foo/bar/baz", false, true),
            ("*/bar/**", "foo/bar/baz/x", true, true),
            ("*/bar/**", "deep/foo/bar/baz/x", false, true),
            ("**/bar/*/*", "deep/foo/bar/baz/x", true, true),
            ("a[c-c]st", "acrt", false, false),
            ("a[c-c]rt", "acrt", true, true),
            ("[!]-]", "]", false, false),
            ("[!]-]", "a", true, true),
            ("\\", "", false, false),
            ("*/\\\\", "/\\", true, true),
            ("[[]", "[", true, true),
            ("[", "[", false, false),
            ("[*", "[abc", false, false),
            ("*/*/*", "foo/bba/arr", true, true),
            ("*/*/*", "foo/bb/aa/rr", false, true),
            ("**/**/**", "foo/bb/aa/rr", true, true),
            ("*X*i", "abcXdefXghi", true, true),
            ("*X*i", "ab/cXd/efXg/hi", false, true),
            ("*/*X*/*/*i", "ab/cXd/efXg/hi", true, true),
            ("**/*X*/**/*i", "ab/cXd/efXg/hi", true, true),
            ("[[:alpha:]][[:digit:]][[:upper:]]", "a1B", true, true),
            ("[[:digit:][:upper:][:space:]]", "a", false, false),
            ("[[:digit:][:upper:][:space:]]", "A", true, true),
            ("[[:digit:][:upper:][:space:]]", "1", true, true),
            ("[[:digit:][:upper:][:spaci:]]", "1", false, false),
            ("[[:xdigit:]]", "5", true, true),
            ("[[:xdigit:]]", "f", true, true),
            ("[a-c[:digit:]x-z]", "5", true, true),
            ("[a-c[:digit:]x-z]", "q", false, false),
            (
                "-*-*-*-*-*-*-12-*-*-*-m-*-*-*",
                "-adobe-courier-bold-o-normal--12-120-75-75-m-70-iso8859-1",
                true,
                true,
            ),
            (
                "XXX/*/*/*/*/*/*/12/*/*/*/m/*/*/*",
                "XXX/adobe/courier/bold/o/normal//12/120/75/75/X/70/iso8859/1",
                false,
                false,
            ),
            (
                "**/*a*b*g*n*t",
                "abcd/abcdefg/abcdefghijk/abcdefghijklmnop.txt",
                true,
                true,
            ),
            (
                "**/*a*b*g*n*t",
                "abcd/abcdefg/abcdefghijk/abcdefghijklmnop.txtz",
                false,
                false,
            ),
        ];
        for &(pattern, text, with, _) in cases {
            assert_eq!(
                path(pattern, text),
                with,
                "{pattern} against {text}, as a path"
            );
        }
    }

    #[test]
    fn folds_case_when_asked() {
        assert!(matches(b"*.TXT", b"a.txt", true, true));
        assert!(!matches(b"*.TXT", b"a.txt", true, false));
    }
}
