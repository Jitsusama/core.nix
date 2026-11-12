//! What Chrome has open now, from its extension's host where it answers
//! and otherwise its own session file, with no process started (open.rs,
//! session.rs), and which of its pages are pages of the web the launcher
//! gives a row: a library the programs daemon reads Chrome's open pages
//! with as they change, and chrome-recent-sites reads its history's sites
//! by, so one rule says which pages are rows. Session files written as
//! Chrome writes them, for tests, come with the `written` feature
//! (written.rs).

pub mod open;
pub mod session;
#[cfg(any(test, feature = "written"))]
pub mod written;

/// Sites left out: Google's own windows, which are programs of their own,
/// signing in, and this machine.
const LEFT_OUT: [&str; 9] = [
    "mail.google.com",
    "calendar.google.com",
    "meet.google.com",
    "drive.google.com",
    "docs.google.com",
    "accounts.google.com",
    "localhost",
    "127.0.0.1",
    "::1",
];

/// The host a web address names, or none when it isn't one.
#[must_use]
pub fn host(address: &str) -> Option<&str> {
    let rest = address
        .strip_prefix("https://")
        .or_else(|| address.strip_prefix("http://"))?;
    let authority = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    let host = match authority.strip_prefix('[') {
        Some(bracketed) => &bracketed[..bracketed.find(']')?],
        None => authority.split(':').next().unwrap_or(authority),
    };
    (!host.is_empty()).then_some(host)
}

/// Whether an address is a page of the web with no program of its own, so
/// the launcher gives it a row (`LEFT_OUT`).
#[must_use]
pub fn on_the_web(address: &str) -> bool {
    host(address)
        .map(str::to_ascii_lowercase)
        .is_some_and(|host| !LEFT_OUT.contains(&host.as_str()))
}

/// Text said whole on one line, its runs of space and line breaks one
/// space, so nothing in a title may break the line it's printed on.
#[must_use]
pub fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_host_is_found_in_any_web_address() {
        assert_eq!(
            host("https://user@Example.org:443/a"),
            Some("Example.org"),
            "past who and the port"
        );
        assert_eq!(host("http://[::1]:8000/"), Some("::1"), "in brackets");
        assert_eq!(
            host("https://example.org?q"),
            Some("example.org"),
            "before a query"
        );
        assert_eq!(host("file:///etc/hosts"), None, "a file is no host's");
        assert_eq!(host("https:///nothing"), None, "an empty host is none");
    }

    #[test]
    fn a_page_with_a_program_of_its_own_or_off_this_machine_isnt_the_webs() {
        assert!(on_the_web("https://example.org/"), "a page of the web");
        assert!(
            !on_the_web("https://Mail.Google.com/mail/u/0/"),
            "Mail has its own"
        );
        assert!(!on_the_web("http://localhost:8080/"), "this machine's own");
        assert!(!on_the_web("chrome://newtab/"), "Chrome's own page");
    }
}
