//! Whether a network reaches beyond itself, or keeps its guests on a
//! sign-in page until they agree to its terms.
//!
//! The check asks Google's `generate_204` page for its empty answer over
//! plain HTTP, which a network that wants a sign-in can't pass along
//! untouched: it redirects the request to its sign-in page, or answers
//! with that page in its place. When a card first reaches a network, it
//! asks Google's page over HTTPS too, which finds the networks that let
//! the plain check through and nothing else. A network that says where to
//! ask it, as RFC 8910 lets it, is asked first, and its answer says when
//! its session ends.
//!
//! wifid.conf says which pages to ask:
//!
//! ```ini
//! [Check]
//! Page=http://connectivitycheck.gstatic.com/generate_204
//! SecurePage=https://www.google.com/generate_204
//! ```

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use rustls::ClientConfig;
use serde_json::Value;

use crate::page::{self, Answer, Page};
use crate::syntax::Document;

/// Checks networks with the pages wifid.conf names.
pub struct Checker {
    pub settings: Settings,
    /// What TLS trusts, without which only the plain check is asked.
    pub tls: Option<Arc<ClientConfig>>,
}

impl Checker {
    /// Checks the network a card is on, through that card alone, finding
    /// each page with the network's own DNS servers; the HTTPS check only
    /// when the card has just reached it. Says when to check it again.
    pub async fn check(
        &self,
        card: &str,
        dns: &[IpAddr],
        advertised: Option<&str>,
        reached: bool,
    ) -> (Verdict, Duration) {
        let tls = self.tls.as_ref();
        let asked = |page: &Page| {
            let page = page.clone();
            async move {
                let fetched = twice(|| page::fetch(card, dns, &page, "*/*", tls), AGAIN);
                match fetched.await {
                    Ok(answer) => verdict(&answer, &page),
                    Err(_) => Verdict::Limited,
                }
            }
        };
        let session = async {
            let api = Page::parse(advertised?)?;
            let answer = page::fetch(card, dns, &api, Session::KIND, tls)
                .await
                .ok()?;
            Session::read(&answer)
        };
        let secure = async {
            if reached && tls.is_some() {
                Some(asked(&self.settings.secure_page).await)
            } else {
                None
            }
        };
        let (session, plain, secure) = tokio::join!(session, asked(&self.settings.page), secure);
        let verdict = judge(
            &self.settings.page,
            session.as_ref(),
            &plain,
            secure.as_ref(),
        );
        let after = recheck_after(&verdict, session.as_ref());
        (verdict, after)
    }
}

/// How long a check waits before asking again for a page it couldn't
/// reach at all.
const AGAIN: Duration = Duration::from_secs(1);

/// Asks once, and again a moment later if that couldn't be done at all:
/// a page out of reach for an instant, as a card's first moments on a
/// network can leave it, says nothing of how far the network reaches.
async fn twice<T, E, F>(mut ask: impl FnMut() -> F, pause: Duration) -> Result<T, E>
where
    F: Future<Output = Result<T, E>>,
{
    if let Ok(answer) = ask().await {
        return Ok(answer);
    }
    tokio::time::sleep(pause).await;
    ask().await
}

/// The pages a check asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Over plain HTTP, which a network wanting a sign-in can't pass along.
    pub page: Page,
    /// Over HTTPS, which one letting the plain check through can't either.
    pub secure_page: Page,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            page: Page::parse("http://connectivitycheck.gstatic.com/generate_204").unwrap(),
            secure_page: Page::parse("https://www.google.com/generate_204").unwrap(),
        }
    }
}

impl Settings {
    /// Reads wifid.conf, where any page it doesn't name is Google's.
    pub fn read(text: &str) -> Result<Settings, String> {
        let document = Document::parse(text).map_err(|malformed| malformed.to_string())?;
        let defaults = Settings::default();
        let page = |key: &str, secure: bool, default: Page| match document.value("Check", key) {
            None => Ok(default),
            Some(address) => match Page::parse(address) {
                Some(page) if page.secure == secure => Ok(page),
                Some(_) if secure => Err(format!("{key}={address} has to be an https address")),
                Some(_) => Err(format!("{key}={address} has to be an http address")),
                None => Err(format!("{key}={address} isn't a web page's address")),
            },
        };
        Ok(Settings {
            page: page("Page", false, defaults.page)?,
            secure_page: page("SecurePage", true, defaults.secure_page)?,
        })
    }
}

/// What the check found of a network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// It reaches beyond itself.
    Online,
    /// It wants a sign-in, on this page.
    SignIn { page: String },
    /// It reaches no further, or nothing sensible answered.
    Limited,
}

/// Reads the check page's answer, which is empty when nothing stood in the
/// way.
pub fn verdict(answer: &[u8], page: &Page) -> Verdict {
    let Some(answer) = Answer::read(answer) else {
        return Verdict::Limited;
    };
    // A proxy between may make the empty answer an empty 200.
    let empty = answer.body.is_empty() || answer.header("Content-Length") == Some("0");
    match answer.code {
        204 => Verdict::Online,
        200 if empty => Verdict::Online,
        300..=399 => match answer.header("Location") {
            Some(location) => Verdict::SignIn {
                page: page.joined(location),
            },
            None => Verdict::Limited,
        },
        200..=299 => Verdict::SignIn {
            page: page.to_string(),
        },
        _ => Verdict::Limited,
    }
}

/// What a network says of the session it gave the machine, from the API
/// RFC 8908 describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// Whether it still wants a sign-in.
    pub captive: bool,
    /// Where to sign in.
    pub page: Option<String>,
    /// How long until the session ends.
    pub remaining: Option<Duration>,
}

impl Session {
    /// The JSON RFC 8908 asks for.
    pub const KIND: &str = "application/captive+json";

    /// Reads the network's answer; none when it isn't one.
    pub fn read(answer: &[u8]) -> Option<Session> {
        let answer = Answer::read(answer).filter(|answer| answer.code == 200)?;
        let said: Value = serde_json::from_slice(answer.body).ok()?;
        Some(Session {
            captive: said["captive"].as_bool()?,
            page: said["user-portal-url"].as_str().map(str::to_string),
            remaining: said["seconds-remaining"].as_u64().map(Duration::from_secs),
        })
    }
}

/// One verdict from all the check heard: the network's own word, the plain
/// check, and the HTTPS one when it was asked. A sign-in page found by any
/// of them is the verdict, then the network's word that it wants none.
/// A network that wants a sign-in without saying where gets one where the
/// check page, asked, went.
pub fn judge(
    asked: &Page,
    session: Option<&Session>,
    plain: &Verdict,
    secure: Option<&Verdict>,
) -> Verdict {
    let found = [Some(plain), secure]
        .into_iter()
        .flatten()
        .find_map(|verdict| match verdict {
            Verdict::SignIn { page } => Some(page.clone()),
            _ => None,
        });
    match (session, found) {
        (
            Some(Session {
                captive: true,
                page: Some(page),
                ..
            }),
            _,
        ) => Verdict::SignIn { page: page.clone() },
        (Some(Session { captive: true, .. }), found) => Verdict::SignIn {
            page: found.unwrap_or_else(|| asked.to_string()),
        },
        (_, Some(page)) => Verdict::SignIn { page },
        (Some(_), None) => Verdict::Online,
        (None, None) if *plain == Verdict::Online && secure != Some(&Verdict::Limited) => {
            Verdict::Online
        }
        (None, None) => Verdict::Limited,
    }
}

/// How long to leave a network before checking it again: soon while it
/// wants a sign-in or reaches no further, since either is waiting on
/// something; otherwise every five minutes, which notices a day pass
/// running out, or as its session ends when it says so.
pub fn recheck_after(verdict: &Verdict, session: Option<&Session>) -> Duration {
    const SOON: Duration = Duration::from_secs(30);
    const USUALLY: Duration = Duration::from_secs(300);
    match (verdict, session.and_then(|session| session.remaining)) {
        (Verdict::Online, Some(remaining)) => USUALLY.min(remaining + Duration::from_secs(1)),
        (Verdict::Online, None) => USUALLY,
        _ => SOON,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn a_page_out_of_reach_is_asked_again() {
        let asked = Cell::new(0);

        let answer: Result<&str, &str> = twice(
            || {
                asked.set(asked.get() + 1);
                let now = asked.get();
                async move { if now == 1 { Err("refused") } else { Ok("204") } }
            },
            Duration::ZERO,
        )
        .await;

        assert_eq!(answer, Ok("204"));
        assert_eq!(asked.get(), 2);
    }

    #[tokio::test]
    async fn a_page_reached_is_asked_once() {
        let asked = Cell::new(0);

        let answer: Result<&str, &str> = twice(
            || {
                asked.set(asked.get() + 1);
                async { Ok("302") }
            },
            Duration::ZERO,
        )
        .await;

        assert_eq!(answer, Ok("302"));
        assert_eq!(asked.get(), 1);
    }

    #[tokio::test]
    async fn a_page_out_of_reach_twice_says_why() {
        let answer: Result<(), &str> = twice(|| async { Err("refused") }, Duration::ZERO).await;

        assert_eq!(answer, Err("refused"));
    }

    fn check_page() -> Page {
        Settings::default().page
    }

    fn signing_in(page: &str) -> Verdict {
        Verdict::SignIn {
            page: page.to_string(),
        }
    }

    #[test]
    fn wifid_conf_names_the_pages_to_ask() {
        let text = "[Check]\nPage=http://check.inn/204\nSecurePage=https://check.inn/204\n";

        let settings = Settings::read(text).unwrap();

        assert_eq!(settings.page, Page::parse("http://check.inn/204").unwrap());
        assert_eq!(
            settings.secure_page,
            Page::parse("https://check.inn/204").unwrap()
        );
    }

    #[test]
    fn a_page_wifid_conf_doesnt_name_is_google_s() {
        let settings = Settings::read("[Check]\nPage=http://check.inn/204\n").unwrap();

        assert_eq!(settings.secure_page, Settings::default().secure_page);
        assert_eq!(Settings::read("").unwrap(), Settings::default());
    }

    #[test]
    fn a_page_of_the_wrong_kind_or_none_at_all_is_refused() {
        let secure_plain_page = Settings::read("[Check]\nPage=https://check.inn/204\n");
        let plain_secure_page = Settings::read("[Check]\nSecurePage=http://check.inn/204\n");
        let nonsense = Settings::read("[Check]\nPage=check.inn\n");

        assert!(secure_plain_page.unwrap_err().contains("Page"));
        assert!(plain_secure_page.unwrap_err().contains("SecurePage"));
        assert!(nonsense.unwrap_err().contains("check.inn"));
    }

    #[test]
    fn an_empty_answer_means_the_network_reaches_beyond_itself() {
        let answer = b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n";

        assert_eq!(verdict(answer, &check_page()), Verdict::Online);
    }

    #[test]
    fn an_empty_answer_a_proxy_called_ok_means_the_same() {
        let answer = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";

        assert_eq!(verdict(answer, &check_page()), Verdict::Online);
    }

    #[test]
    fn a_redirect_is_to_the_sign_in_page() {
        let answer = b"HTTP/1.1 302 Found\r\nlocation: https://front-desk.inn/terms?mac=1\r\n\r\n";
        let nearby = b"HTTP/1.0 307 Temporary Redirect\r\nLocation: /sign-in\r\n\r\n";

        assert_eq!(
            verdict(answer, &check_page()),
            signing_in("https://front-desk.inn/terms?mac=1")
        );
        assert_eq!(
            verdict(nearby, &check_page()),
            signing_in("http://connectivitycheck.gstatic.com/sign-in")
        );
    }

    #[test]
    fn a_page_in_place_of_the_empty_answer_is_the_sign_in_page() {
        let answer = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html>Welcome</html>";

        assert_eq!(
            verdict(answer, &check_page()),
            signing_in("http://connectivitycheck.gstatic.com/generate_204")
        );
    }

    #[test]
    fn an_error_or_anything_but_http_means_the_network_reaches_no_further() {
        let error = b"HTTP/1.1 503 Service Unavailable\r\n\r\nGone fishing";
        let lost = b"HTTP/1.1 302 Found\r\n\r\n";
        let nonsense = b"SSH-2.0-OpenSSH_9.9\r\n";

        assert_eq!(verdict(error, &check_page()), Verdict::Limited);
        assert_eq!(verdict(lost, &check_page()), Verdict::Limited);
        assert_eq!(verdict(nonsense, &check_page()), Verdict::Limited);
    }

    fn api(body: &Value) -> Vec<u8> {
        let mut answer =
            b"HTTP/1.0 200 OK\r\nContent-Type: application/captive+json\r\n\r\n".to_vec();
        answer.extend(serde_json::to_vec(body).unwrap());
        answer
    }

    #[test]
    fn the_network_says_whether_it_wants_a_sign_in_and_where() {
        let wants = api(&json!({"captive": true, "user-portal-url": "https://front-desk.inn/"}));

        assert_eq!(
            Session::read(&wants),
            Some(Session {
                captive: true,
                page: Some("https://front-desk.inn/".to_string()),
                remaining: None,
            })
        );
    }

    #[test]
    fn the_network_says_how_long_the_session_it_gave_lasts() {
        let given =
            api(&json!({"captive": false, "seconds-remaining": 326, "bytes-remaining": 65536}));

        let session = Session::read(&given).unwrap();

        assert!(!session.captive);
        assert_eq!(session.remaining, Some(Duration::from_secs(326)));
    }

    #[test]
    fn an_answer_that_isnt_the_network_s_word_is_none() {
        let error = b"HTTP/1.0 404 Not Found\r\n\r\n{\"captive\": false}";
        let page = b"HTTP/1.0 200 OK\r\n\r\n<html>";
        let unsaid = api(&json!({"user-portal-url": "https://front-desk.inn/"}));

        assert_eq!(Session::read(error), None);
        assert_eq!(Session::read(page), None);
        assert_eq!(Session::read(&unsaid), None);
    }

    fn session(captive: bool, page: Option<&str>) -> Session {
        Session {
            captive,
            page: page.map(str::to_string),
            remaining: None,
        }
    }

    #[test]
    fn a_sign_in_page_found_any_way_is_the_verdict() {
        let inn = "http://front-desk.inn/";
        let wanting = session(true, Some(inn));
        let not_wanting = session(false, None);

        let asked = check_page();

        let wanted = judge(
            &asked,
            Some(&wanting),
            &Verdict::Online,
            Some(&Verdict::Online),
        );
        let found_anyway = judge(&asked, Some(&not_wanting), &signing_in(inn), None);
        let found = judge(&asked, None, &signing_in(inn), Some(&Verdict::Limited));

        assert_eq!(wanted, signing_in(inn));
        assert_eq!(found_anyway, signing_in(inn));
        assert_eq!(found, signing_in(inn));
    }

    #[test]
    fn a_network_wanting_a_sign_in_without_saying_where_is_signed_in_to_where_the_check_went() {
        let wanting = session(true, None);
        let inn = "http://front-desk.inn/";

        let asked = check_page();

        let found = judge(&asked, Some(&wanting), &signing_in(inn), None);
        let unfound = judge(&asked, Some(&wanting), &Verdict::Online, None);

        assert_eq!(found, signing_in(inn));
        assert_eq!(
            unfound,
            signing_in("http://connectivitycheck.gstatic.com/generate_204")
        );
    }

    #[test]
    fn the_network_s_word_that_it_wants_none_is_taken() {
        let not_wanting = session(false, None);

        let judged = judge(
            &check_page(),
            Some(&not_wanting),
            &Verdict::Limited,
            Some(&Verdict::Limited),
        );

        assert_eq!(judged, Verdict::Online);
    }

    #[test]
    fn without_it_both_checks_must_get_through() {
        let asked = check_page();
        let judged =
            |plain: Verdict, secure: Option<Verdict>| judge(&asked, None, &plain, secure.as_ref());

        assert_eq!(
            judged(Verdict::Online, Some(Verdict::Online)),
            Verdict::Online
        );
        assert_eq!(judged(Verdict::Online, None), Verdict::Online);
        assert_eq!(
            judged(Verdict::Online, Some(Verdict::Limited)),
            Verdict::Limited
        );
        assert_eq!(
            judged(Verdict::Limited, Some(Verdict::Online)),
            Verdict::Limited
        );
    }

    #[test]
    fn a_network_waiting_on_something_is_checked_again_soon() {
        let soon = Duration::from_secs(30);

        assert_eq!(
            recheck_after(&signing_in("http://front-desk.inn/"), None),
            soon
        );
        assert_eq!(recheck_after(&Verdict::Limited, None), soon);
    }

    #[test]
    fn a_network_reaching_beyond_itself_is_checked_every_five_minutes_or_as_its_session_ends() {
        let ending = Session {
            remaining: Some(Duration::from_secs(90)),
            ..session(false, None)
        };
        let lasting = Session {
            remaining: Some(Duration::from_secs(3600)),
            ..session(false, None)
        };

        assert_eq!(
            recheck_after(&Verdict::Online, None),
            Duration::from_secs(300)
        );
        assert_eq!(
            recheck_after(&Verdict::Online, Some(&ending)),
            Duration::from_secs(91)
        );
        assert_eq!(
            recheck_after(&Verdict::Online, Some(&lasting)),
            Duration::from_secs(300)
        );
    }
}
