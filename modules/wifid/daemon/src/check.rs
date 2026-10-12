//! Whether a network reaches beyond itself, or keeps its guests on a
//! sign-in page until they agree to its terms.
//!
//! The check asks Google's generate_204 page for its empty answer over
//! plain HTTP, which a network that wants a sign-in can't pass along
//! untouched: it redirects the request to its sign-in page, or answers
//! with that page in its place.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpSocket;
use tokio::time::timeout;

use crate::lookup;

/// How long the check page has to answer, all told.
const PATIENCE: Duration = Duration::from_secs(10);

/// More than any sign-in page needs to be told from the empty answer.
const ENOUGH: u64 = 64 * 1024;

/// Checks the network a card is on, through that card alone, finding the
/// check page with the network's own DNS servers.
pub async fn check(card: &str, dns: &[IpAddr]) -> Verdict {
    for server in dns {
        match lookup::addresses(card, *server, HOST).await {
            Ok(addresses) if !addresses.is_empty() => {
                return match timeout(PATIENCE, fetch(card, addresses[0])).await {
                    Ok(Ok(answer)) => verdict(&answer),
                    _ => Verdict::Limited,
                };
            }
            _ => continue,
        }
    }
    Verdict::Limited
}

/// The check page's answer from address, as it came.
async fn fetch(card: &str, address: IpAddr) -> io::Result<Vec<u8>> {
    let socket = match address {
        IpAddr::V4(_) => TcpSocket::new_v4()?,
        IpAddr::V6(_) => TcpSocket::new_v6()?,
    };
    socket.bind_device(Some(card.as_bytes()))?;
    let mut stream = socket.connect(SocketAddr::new(address, 80)).await?;
    stream.write_all(&request()).await?;
    let mut answer = Vec::new();
    stream.take(ENOUGH).read_to_end(&mut answer).await?;
    Ok(answer)
}

/// The check page, which answers 204 with nothing in it.
pub const HOST: &str = "connectivitycheck.gstatic.com";
pub const PATH: &str = "/generate_204";

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

/// The request for the check page.
pub fn request() -> Vec<u8> {
    format!("GET {PATH} HTTP/1.1\r\nHost: {HOST}\r\nConnection: close\r\n\r\n").into_bytes()
}

/// Reads the check page's answer, as it came over the connection.
pub fn verdict(answer: &[u8]) -> Verdict {
    let Some(end) = answer.windows(4).position(|window| window == b"\r\n\r\n") else {
        return Verdict::Limited;
    };
    let head = String::from_utf8_lossy(&answer[..end]);
    let body = &answer[end + 4..];
    let status = head.split("\r\n").next().unwrap_or_default();
    let Some(code) = status_code(status) else {
        return Verdict::Limited;
    };
    let header = |wanted: &str| {
        let mut headers = head.split("\r\n").skip(1);
        headers.find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case(wanted)
                .then(|| value.trim().to_string())
        })
    };
    // A proxy between may make the empty answer an empty 200.
    let empty = body.is_empty() || header("Content-Length").as_deref() == Some("0");
    match code {
        204 => Verdict::Online,
        200 if empty => Verdict::Online,
        300..=399 => match header("Location") {
            Some(location) => Verdict::SignIn {
                page: absolute(&location),
            },
            None => Verdict::Limited,
        },
        200..=299 => Verdict::SignIn {
            page: format!("http://{HOST}{PATH}"),
        },
        _ => Verdict::Limited,
    }
}

fn status_code(status: &str) -> Option<u16> {
    let (version, rest) = status.split_once(' ')?;
    if !version.starts_with("HTTP/") {
        return None;
    }
    rest.get(..3)?.parse().ok()
}

/// Where a redirect points, read from the check page's own address.
fn absolute(location: &str) -> String {
    if location.contains("://") {
        location.to_string()
    } else if location.starts_with("//") {
        format!("http:{location}")
    } else if location.starts_with('/') {
        format!("http://{HOST}{location}")
    } else {
        format!("http://{HOST}/{location}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_request_asks_for_the_check_page_and_nothing_kept() {
        let request = String::from_utf8(request()).unwrap();

        assert_eq!(
            request,
            "GET /generate_204 HTTP/1.1\r\n\
             Host: connectivitycheck.gstatic.com\r\n\
             Connection: close\r\n\
             \r\n"
        );
    }

    #[test]
    fn an_empty_answer_means_the_network_reaches_beyond_itself() {
        let answer = b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n";

        assert_eq!(verdict(answer), Verdict::Online);
    }

    #[test]
    fn an_empty_answer_a_proxy_called_ok_means_the_same() {
        let answer = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";

        assert_eq!(verdict(answer), Verdict::Online);
    }

    #[test]
    fn a_redirect_is_to_the_sign_in_page() {
        let answer = b"HTTP/1.1 302 Found\r\nlocation: https://front-desk.inn/terms?mac=1\r\n\r\n";

        assert_eq!(
            verdict(answer),
            Verdict::SignIn {
                page: "https://front-desk.inn/terms?mac=1".to_string()
            }
        );
    }

    #[test]
    fn a_redirect_within_the_check_s_site_is_read_as_on_it() {
        let answer = b"HTTP/1.0 307 Temporary Redirect\r\nLocation: /sign-in\r\n\r\n";

        assert_eq!(
            verdict(answer),
            Verdict::SignIn {
                page: "http://connectivitycheck.gstatic.com/sign-in".to_string()
            }
        );
    }

    #[test]
    fn a_page_in_place_of_the_empty_answer_is_the_sign_in_page() {
        let answer = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html>Welcome</html>";

        assert_eq!(
            verdict(answer),
            Verdict::SignIn {
                page: "http://connectivitycheck.gstatic.com/generate_204".to_string()
            }
        );
    }

    #[test]
    fn an_error_or_anything_but_http_means_the_network_reaches_no_further() {
        let error = b"HTTP/1.1 503 Service Unavailable\r\n\r\nGone fishing";
        let nonsense = b"SSH-2.0-OpenSSH_9.9\r\n";
        let nothing = b"";

        assert_eq!(verdict(error), Verdict::Limited);
        assert_eq!(verdict(nonsense), Verdict::Limited);
        assert_eq!(verdict(nothing), Verdict::Limited);
    }
}
