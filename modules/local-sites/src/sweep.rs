//! What a document could have left on the sites' origin while the two
//! shared it, and how a browser is rid of it. A worker it installed stands
//! between every later page and the server, answering a navigation before
//! the server hears of it, and a refused update keeps it standing; so a
//! worker's script is answered, under either name, with one that lets go
//! of itself, which retires an old worker at its next update check and
//! keeps a new one from standing. What it stored is cleared once, by the
//! first page of the sites' own a browser navigates to, marked done by a
//! cookie no other host can set.

/// The script every worker's script is answered with: it takes its place
/// at once and lets go of its registration, so no page keeps a worker.
pub const LETTING_GO: &[u8] = b"// No page keeps a worker here, so this one lets go of itself.\n\
self.addEventListener(\"install\", () => self.skipWaiting());\n\
self.addEventListener(\"activate\", (event) => event.waitUntil(self.registration.unregister()));\n";

/// The cookie saying a browser's storage was swept of what a document could
/// have left there, so it's swept once.
const SWEPT: &str = "__Host-sites-swept=1";

/// The sweep's cookie as it's set: for this name alone, every path, a year,
/// read by no script. `__Host-` with `Secure` (which a browser holds to on
/// a loopback name over plain HTTP) keeps another host from setting it, as
/// the documents' name could set a plain one for `sites.localhost` and so
/// keep the sweep from ever running. Lax rather than Strict, since a
/// navigation from a sandboxed file carries no Strict cookie, and the sweep
/// would then recur, wiping the theme and Code's room each time.
pub const SWEEPING: &str =
    "__Host-sites-swept=1; Secure; Path=/; Max-Age=31536000; SameSite=Lax; HttpOnly";

/// What the browser says of a request beside its cookies: where it came
/// from and what it's for, as `Sec-Fetch-Site` and `Sec-Fetch-Dest` say.
pub struct Asked<'a> {
    pub cookies: Option<&'a str>,
    pub site: Option<&'a str>,
    pub dest: Option<&'a str>,
}

/// Whether a page of the sites' own, asked for as `asked` says, is sent
/// with the sweep: for a navigation of Joel's own that isn't marked swept,
/// never for another site's, which would wipe what the sites keep from a
/// picture of it. A browser saying neither, as a test's socket, is swept.
pub fn due(asked: &Asked) -> bool {
    let swept = asked
        .cookies
        .is_some_and(|cookies| cookies.split(';').any(|cookie| cookie.trim() == SWEPT));
    let navigated = asked.dest.is_none_or(|dest| dest == "document");
    let own = asked.site.is_none_or(|site| site != "cross-site");
    !swept && navigated && own
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A navigation as a browser asks it, from `site`, with `cookies`.
    fn navigated<'a>(site: &'a str, cookies: Option<&'a str>) -> Asked<'a> {
        Asked {
            cookies,
            site: Some(site),
            dest: Some("document"),
        }
    }

    #[test]
    fn a_navigation_of_joels_own_not_yet_swept_is_swept() {
        for site in ["none", "same-origin", "same-site"] {
            assert!(due(&navigated(site, None)), "from {site}");
        }
        let unsaid = Asked {
            cookies: Some("theme=dark"),
            site: None,
            dest: None,
        };
        assert!(due(&unsaid), "a browser saying neither");
    }

    #[test]
    fn a_browser_marked_swept_is_swept_no_more() {
        let marked = navigated("none", Some("theme=dark; __Host-sites-swept=1"));
        assert!(!due(&marked), "marked by the server's cookie");
    }

    #[test]
    fn a_plain_cookie_another_host_could_set_marks_nothing() {
        let planted = navigated("same-site", Some("sites-swept=1"));
        assert!(due(&planted), "a cookie the documents' name could set");
    }

    #[test]
    fn another_sites_request_or_a_picture_is_never_swept() {
        assert!(!due(&navigated("cross-site", None)), "another site's link");
        let pictured = Asked {
            cookies: None,
            site: Some("same-origin"),
            dest: Some("image"),
        };
        assert!(!due(&pictured), "a picture of a page");
    }
}
