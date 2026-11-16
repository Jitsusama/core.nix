//! Who may come in. Every request names the server by a name its module
//! lists and the port it listens on, so a page elsewhere that points its
//! own name at this machine (DNS rebinding) is refused before anything is
//! read. A feed's WebSocket also says the page it came from, its origin,
//! which must be this server's own: a browser lets any site open a
//! WebSocket to loopback, and only the origin tells Joel's pages from a
//! stranger's.

/// The names and port a request must name the server by.
#[derive(Debug, Clone)]
pub struct Door {
    names: Vec<String>,
    port: u16,
}

impl Door {
    /// A door for the server reached by `names` on `port`.
    pub fn new(names: &[String], port: u16) -> Self {
        Self {
            names: names.iter().map(|name| name.to_ascii_lowercase()).collect(),
            port,
        }
    }

    /// Whether a request's `Host` names this server.
    pub fn hosts(&self, host: &str) -> bool {
        let Some((name, port)) = host.rsplit_once(':') else {
            return false;
        };
        port.parse() == Ok(self.port) && self.names.contains(&name.to_ascii_lowercase())
    }

    /// Whether a WebSocket's `Origin` is a page this server served.
    pub fn admits(&self, origin: &str) -> bool {
        origin
            .strip_prefix("http://")
            .is_some_and(|host| self.hosts(host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn door() -> Door {
        Door::new(
            &["sites.localhost".to_owned(), "127.0.0.1".to_owned()],
            8484,
        )
    }

    #[test]
    fn its_own_names_on_its_own_port_are_hosted() {
        assert!(door().hosts("sites.localhost:8484"));
        assert!(
            door().hosts("Sites.Localhost:8484"),
            "a name's case is the browser's"
        );
        assert!(door().hosts("127.0.0.1:8484"));
    }

    #[test]
    fn another_name_another_port_or_none_is_not() {
        assert!(!door().hosts("evil.example:8484"), "a rebound name");
        assert!(!door().hosts("sites.localhost:9000"));
        assert!(!door().hosts("sites.localhost"), "no port");
    }

    #[test]
    fn only_its_own_pages_are_admitted_to_a_feed() {
        assert!(door().admits("http://sites.localhost:8484"));
        assert!(
            !door().admits("https://sites.localhost:8484"),
            "it serves no https"
        );
        assert!(!door().admits("http://evil.example"));
        assert!(!door().admits("null"), "a file or a sandboxed frame");
    }
}
