//! Who may come in. Every request names the server by a name its module
//! lists and the port it listens on, so a page elsewhere that points its
//! own name at this machine (DNS rebinding) is refused before anything is
//! read. The documents sessions write have a name of their own, and so an
//! origin of their own, which only ever reads; the sites' own pages are
//! served under the others. A feed's WebSocket also says the page it came
//! from, its origin, which must be one of the sites' own: a browser lets any
//! site open a WebSocket to loopback, and only the origin tells Joel's pages
//! from a stranger's, or from a session's document.

/// The names and port a request must name the server by.
#[derive(Debug, Clone)]
pub struct Door {
    names: Vec<String>,
    documents: String,
    port: u16,
}

/// Which of the server's two origins a request names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The sites' own, which Nix and the home's builder made.
    Sites,
    /// The documents sessions write.
    Documents,
}

impl Door {
    /// A door for the server reached by `names`, and by `documents` for
    /// the documents, on `port`.
    pub fn new(names: &[String], documents: &str, port: u16) -> Self {
        Self {
            names: names.iter().map(|name| name.to_ascii_lowercase()).collect(),
            documents: documents.to_ascii_lowercase(),
            port,
        }
    }

    /// Which origin a request's `Host` names, or none where it names
    /// another server.
    pub fn side(&self, host: &str) -> Option<Side> {
        let (name, port) = host.rsplit_once(':')?;
        if port.parse() != Ok(self.port) {
            return None;
        }
        let name = name.to_ascii_lowercase();
        if self.names.contains(&name) {
            Some(Side::Sites)
        } else if name == self.documents {
            Some(Side::Documents)
        } else {
            None
        }
    }

    /// Whether an `Origin` is one of the sites' own pages: never a
    /// document's, a stranger's or a sandboxed frame's.
    pub fn admits(&self, origin: &str) -> bool {
        origin
            .strip_prefix("http://")
            .is_some_and(|host| self.side(host) == Some(Side::Sites))
    }

    /// The address of `path` on the other side's origin from `side`: a
    /// document's on the documents', a page of the sites' own on the first
    /// of their names. Only a path from the root is taken, so the address
    /// leads nowhere but this server.
    pub fn across(&self, side: Side, path: &str) -> Option<String> {
        if !path.starts_with('/') {
            return None;
        }
        let name = match side {
            Side::Sites => &self.documents,
            Side::Documents => self.names.first()?,
        };
        Some(format!("http://{name}:{}{path}", self.port))
    }

    /// The address of `path` under the first of the sites' names, where
    /// `host` names them by another: each name is an origin of its own, with
    /// its own kept look and room, so a browser kept to one sees one set of
    /// them whichever name a visit began at. None where `host` is the first
    /// already, or names no page of the sites' own.
    pub fn settled(&self, host: &str, path: &str) -> Option<String> {
        let first = self.names.first()?;
        let (name, _) = host.rsplit_once(':')?;
        let other = self.side(host) == Some(Side::Sites) && !name.eq_ignore_ascii_case(first);
        (other && path.starts_with('/')).then(|| format!("http://{first}:{}{path}", self.port))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn door() -> Door {
        Door::new(
            &["sites.localhost".to_owned(), "127.0.0.1".to_owned()],
            "documents.sites.localhost",
            8484,
        )
    }

    #[test]
    fn its_own_names_on_its_own_port_are_hosted() {
        assert_eq!(
            door().side("sites.localhost:8484"),
            Some(Side::Sites),
            "its first name"
        );
        assert_eq!(
            door().side("Sites.Localhost:8484"),
            Some(Side::Sites),
            "a name's case is the browser's"
        );
        assert_eq!(
            door().side("127.0.0.1:8484"),
            Some(Side::Sites),
            "its address as a name"
        );
        assert_eq!(
            door().side("Documents.Sites.Localhost:8484"),
            Some(Side::Documents),
            "the documents' name"
        );
    }

    #[test]
    fn another_name_another_port_or_none_is_not() {
        assert_eq!(door().side("evil.example:8484"), None, "a rebound name");
        assert_eq!(door().side("sites.localhost:9000"), None, "another port");
        assert_eq!(door().side("sites.localhost"), None, "no port");
        assert_eq!(
            door().side("documents.sites.localhost:9000"),
            None,
            "the documents' name on another port"
        );
    }

    #[test]
    fn only_its_own_pages_are_admitted_to_a_feed() {
        assert!(door().admits("http://sites.localhost:8484"), "its own page");
        assert!(
            !door().admits("https://sites.localhost:8484"),
            "it serves no https"
        );
        assert!(!door().admits("http://evil.example"), "a stranger's page");
        assert!(!door().admits("null"), "a file or a sandboxed frame");
        assert!(
            !door().admits("http://documents.sites.localhost:8484"),
            "a session's document"
        );
    }

    #[test]
    fn another_of_the_sites_names_settles_on_the_first() {
        assert_eq!(
            door()
                .settled("127.0.0.1:8484", "/notes/?at=2#end")
                .as_deref(),
            Some("http://sites.localhost:8484/notes/?at=2#end"),
            "a visit by another of the sites' names should move to the first, path whole"
        );
        assert_eq!(
            door()
                .settled("127.0.0.1:8484", "//evil.example/")
                .as_deref(),
            Some("http://sites.localhost:8484//evil.example/"),
            "a path that reads as another host should stay on this server"
        );
        assert_eq!(
            door().settled("127.0.0.1:8484", "@evil.example/"),
            None,
            "only a path from the root should be taken"
        );
        for host in [
            "sites.localhost:8484",
            "SITES.localhost:8484",
            "documents.sites.localhost:8484",
            "127.0.0.1:9999",
        ] {
            assert_eq!(
                door().settled(host, "/"),
                None,
                "{host} is the first name, or none of the sites', so stays"
            );
        }
    }

    #[test]
    fn a_path_is_taken_across_whole_and_only_from_the_root() {
        assert_eq!(
            door().across(Side::Sites, "/notes/?at=2#end").as_deref(),
            Some("http://documents.sites.localhost:8484/notes/?at=2#end"),
            "a document goes to the documents' origin, its query kept"
        );
        assert_eq!(
            door().across(Side::Documents, "/mycelium/").as_deref(),
            Some("http://sites.localhost:8484/mycelium/"),
            "a page of the sites' own goes back to theirs"
        );
        assert_eq!(
            door().across(Side::Sites, "//evil.example/").as_deref(),
            Some("http://documents.sites.localhost:8484//evil.example/"),
            "a path that reads as another host stays a path here"
        );
        assert_eq!(
            door().across(Side::Sites, "http://evil.example/"),
            None,
            "a request naming another server whole goes nowhere"
        );
        assert_eq!(
            door().across(Side::Sites, "@evil.example/"),
            None,
            "a path not from the root goes nowhere"
        );
    }
}
