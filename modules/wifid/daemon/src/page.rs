//! The pages a check asks for, and asking for them through one card.
//!
//! Every request is HTTP/1.0, which no server may answer in chunks, so an
//! answer is read whole as it came, and closing the connection ends it.

use std::fmt;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, RootCertStore};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpSocket;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;

use crate::lookup;

/// How long a page has to answer, all told.
const PATIENCE: Duration = Duration::from_secs(10);

/// More than any sign-in page needs to be told from the empty answer.
const ENOUGH: u64 = 64 * 1024;

/// What TLS trusts: the certificate authorities the machine does.
pub fn trusting(bundle: &Path) -> io::Result<Arc<ClientConfig>> {
    let certificates = CertificateDer::pem_file_iter(bundle).map_err(io::Error::other)?;
    let certificates: Vec<_> = certificates
        .collect::<Result<_, _>>()
        .map_err(io::Error::other)?;
    let mut roots = RootCertStore::empty();
    roots.add_parsable_certificates(certificates);
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(io::Error::other)?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

/// The page's answer, asked through card alone, finding its host with the
/// network's own DNS servers.
pub async fn fetch(
    card: &str,
    dns: &[IpAddr],
    page: &Page,
    accept: &str,
    tls: Option<&Arc<ClientConfig>>,
) -> io::Result<Vec<u8>> {
    let asked = async {
        let address = address(card, dns, &page.host).await?;
        let socket = match address {
            IpAddr::V4(_) => TcpSocket::new_v4()?,
            IpAddr::V6(_) => TcpSocket::new_v6()?,
        };
        socket.bind_device(Some(card.as_bytes()))?;
        let connection = socket.connect(SocketAddr::new(address, page.port)).await?;
        let request = page.request(accept);
        if !page.secure {
            return exchange(connection, &request).await;
        }
        let tls = tls.ok_or_else(|| io::Error::other("there are no certificates to trust"))?;
        let name = ServerName::try_from(page.host.clone())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        let connection = TlsConnector::from(Arc::clone(tls))
            .connect(name, connection)
            .await?;
        exchange(connection, &request).await
    };
    let late = || {
        io::Error::new(
            io::ErrorKind::TimedOut,
            format!("{page} didn't answer in time"),
        )
    };
    timeout(PATIENCE, asked).await.map_err(|_| late())?
}

/// The host's address, as the network's own servers give it.
async fn address(card: &str, dns: &[IpAddr], host: &str) -> io::Result<IpAddr> {
    if let Ok(address) = host.parse() {
        return Ok(address);
    }
    for server in dns {
        if let Ok(addresses) = lookup::addresses(card, *server, host).await
            && let Some(address) = addresses.first()
        {
            return Ok(*address);
        }
    }
    let reason = format!("the network's DNS servers didn't give {host}'s address");
    Err(io::Error::new(io::ErrorKind::NotFound, reason))
}

async fn exchange<S>(mut connection: S, request: &[u8]) -> io::Result<Vec<u8>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    connection.write_all(request).await?;
    let mut answer = Vec::new();
    match (&mut connection)
        .take(ENOUGH)
        .read_to_end(&mut answer)
        .await
    {
        Ok(_) => Ok(answer),
        // Plenty of servers close without TLS's goodbye, once they've said
        // all they will.
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof && !answer.is_empty() => {
            Ok(answer)
        }
        Err(error) => Err(error),
    }
}

/// Where a page is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// Whether it's reached over TLS.
    pub secure: bool,
    pub host: String,
    pub port: u16,
    /// The path, with any query.
    pub path: String,
}

impl Page {
    /// Reads an http or https address.
    pub fn parse(address: &str) -> Option<Page> {
        let (secure, rest) = match address.split_once("://")? {
            ("http", rest) => (false, rest),
            ("https", rest) => (true, rest),
            _ => return None,
        };
        let (site, path) = match rest.find(['/', '?']) {
            Some(at) => (&rest[..at], &rest[at..]),
            None => (rest, "/"),
        };
        let path = if path.starts_with('?') {
            format!("/{path}")
        } else {
            path.to_string()
        };
        if site.contains('@') {
            return None;
        }
        let (host, port) = match site.strip_prefix('[') {
            Some(bracketed) => {
                let (host, after) = bracketed.split_once(']')?;
                (host, after.strip_prefix(':'))
            }
            None => match site.split_once(':') {
                Some((host, port)) => (host, Some(port)),
                None => (site, None),
            },
        };
        let port = match port {
            Some(port) => port.parse().ok()?,
            None if secure => 443,
            None => 80,
        };
        if host.is_empty() {
            return None;
        }
        Some(Page {
            secure,
            host: host.to_string(),
            port,
            path,
        })
    }

    /// A page this one's answer points to, read from it the way a browser
    /// would: a whole address, one on another site, or one on this site.
    pub fn joined(&self, location: &str) -> String {
        let scheme = self.scheme();
        if location.contains("://") {
            location.to_string()
        } else if let Some(elsewhere) = location.strip_prefix("//") {
            format!("{scheme}://{elsewhere}")
        } else {
            let site = self.site();
            let path = location.strip_prefix('/').unwrap_or(location);
            format!("{scheme}://{site}/{path}")
        }
    }

    /// The request for this page, accepting what's named.
    pub fn request(&self, accept: &str) -> Vec<u8> {
        let (path, site) = (&self.path, self.site());
        let request = format!(
            "GET {path} HTTP/1.0\r\nHost: {site}\r\nAccept: {accept}\r\nConnection: close\r\n\r\n"
        );
        request.into_bytes()
    }

    fn scheme(&self) -> &'static str {
        if self.secure { "https" } else { "http" }
    }

    /// The host, and the port when it isn't the scheme's own.
    fn site(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        let usual = if self.secure { 443 } else { 80 };
        if self.port == usual {
            host
        } else {
            format!("{host}:{}", self.port)
        }
    }
}

impl fmt::Display for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}{}", self.scheme(), self.site(), self.path)
    }
}

/// A page's answer, as it came over the connection.
#[derive(Debug)]
pub struct Answer<'a> {
    pub code: u16,
    head: String,
    pub body: &'a [u8],
}

impl<'a> Answer<'a> {
    /// Reads an answer; none for anything but HTTP.
    pub fn read(answer: &'a [u8]) -> Option<Answer<'a>> {
        let end = answer.windows(4).position(|window| window == b"\r\n\r\n")?;
        let head = String::from_utf8_lossy(&answer[..end]).into_owned();
        let status = head.split("\r\n").next()?;
        let (version, rest) = status.split_once(' ')?;
        if !version.starts_with("HTTP/") {
            return None;
        }
        let code = rest.get(..3)?.parse().ok()?;
        Some(Answer {
            code,
            head,
            body: &answer[end + 4..],
        })
    }

    /// A header's value, whatever case its name is written in.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.head.split("\r\n").skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case(name)
                .then_some(value.trim())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(address: &str) -> Page {
        Page::parse(address).unwrap()
    }

    #[test]
    fn an_address_is_read_into_where_the_page_is() {
        let plain = page("http://connectivitycheck.gstatic.com/generate_204");
        let secure = page("https://api.lantern:8443/captive?id=7");

        assert_eq!(
            plain,
            Page {
                secure: false,
                host: "connectivitycheck.gstatic.com".to_string(),
                port: 80,
                path: "/generate_204".to_string(),
            }
        );
        assert_eq!(
            secure,
            Page {
                secure: true,
                host: "api.lantern".to_string(),
                port: 8443,
                path: "/captive?id=7".to_string(),
            }
        );
    }

    #[test]
    fn an_address_with_no_path_or_an_ipv6_host_is_read_too() {
        assert_eq!(page("https://www.google.com").path, "/");
        assert_eq!(page("http://[fd00::1]:8080/x").host, "fd00::1");
        assert_eq!(page("http://[fd00::1]:8080/x").port, 8080);
    }

    #[test]
    fn an_address_that_isnt_a_web_page_s_is_refused() {
        for address in [
            "ftp://inn/",
            "front-desk.inn",
            "http://",
            "http://inn:port/",
            "http://me@inn/",
        ] {
            assert_eq!(Page::parse(address), None, "{address}");
        }
    }

    #[test]
    fn a_page_is_written_as_its_address() {
        for address in [
            "http://inn/sign-in",
            "https://api.lantern:8443/captive",
            "http://[fd00::1]:8080/x",
        ] {
            assert_eq!(page(address).to_string(), address);
        }
    }

    #[test]
    fn a_page_pointed_to_is_read_from_the_page_that_pointed() {
        let check = page("http://connectivitycheck.gstatic.com/generate_204");

        assert_eq!(
            check.joined("https://front-desk.inn/terms"),
            "https://front-desk.inn/terms"
        );
        assert_eq!(
            check.joined("//front-desk.inn/terms"),
            "http://front-desk.inn/terms"
        );
        assert_eq!(
            check.joined("/sign-in"),
            "http://connectivitycheck.gstatic.com/sign-in"
        );
        assert_eq!(
            check.joined("sign-in"),
            "http://connectivitycheck.gstatic.com/sign-in"
        );
    }

    #[test]
    fn the_request_asks_for_the_page_and_nothing_kept() {
        let request = page("https://api.lantern:8443/captive").request("application/captive+json");

        assert_eq!(
            String::from_utf8(request).unwrap(),
            "GET /captive HTTP/1.0\r\n\
             Host: api.lantern:8443\r\n\
             Accept: application/captive+json\r\n\
             Connection: close\r\n\
             \r\n"
        );
    }

    #[test]
    fn an_answer_is_read_into_its_code_headers_and_body() {
        let raw = b"HTTP/1.1 302 Found\r\nlocation: /sign-in\r\n\r\n<html>";

        let answer = Answer::read(raw).unwrap();

        assert_eq!(answer.code, 302);
        assert_eq!(answer.header("Location"), Some("/sign-in"));
        assert_eq!(answer.header("Content-Type"), None);
        assert_eq!(answer.body, b"<html>");
    }

    #[test]
    fn anything_but_an_http_answer_is_none() {
        assert!(Answer::read(b"SSH-2.0-OpenSSH_9.9\r\n").is_none());
        assert!(Answer::read(b"HTTP/1.1 204").is_none());
        assert!(Answer::read(b"").is_none());
    }
}
