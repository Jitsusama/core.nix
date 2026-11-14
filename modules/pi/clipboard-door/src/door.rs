//! One client of the door answered: who it is, what it asks, and the
//! line the journal keeps of it.

use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use crate::desk;
use crate::kinds;
use crate::peer;
use crate::said;
use crate::wire::{Answer, LONGEST_LINE, Request, Selection, Wanted, Why, plain_kind};

/// The most a copy carries, and a paste reads, in bytes: a large
/// screenshot, with room to spare.
pub const LARGEST: usize = 64 << 20;

/// How long a client has between one byte of its request and the next,
/// so one that never finishes asking never holds a place.
const PATIENCE: Duration = Duration::from_secs(10);

/// The desktop's two selections, as the door reaches them: the
/// compositor's own (src/desk.rs), or a stand-in for tests.
pub trait Clipboard {
    /// The types `selection` is offered as now.
    fn types(&self, selection: Selection) -> Result<Vec<String>, Why>;
    /// What `selection` holds, as `wanted`, up to `largest` bytes, never
    /// a password manager's secret.
    fn paste(
        &self,
        selection: Selection,
        wanted: &Wanted,
        largest: usize,
    ) -> Result<desk::Pasted, Why>;
    /// Empty `selection`.
    fn clear(&self, selection: Selection) -> Result<(), Why>;
    /// Put `bytes` on `selection`, offered as `types`.
    fn copy(
        &self,
        selection: Selection,
        types: &[String],
        bytes: Vec<u8>,
        once: bool,
    ) -> Result<(), Why>;
}

/// The compositor's own selections.
pub struct Desktop;

impl Clipboard for Desktop {
    fn types(&self, selection: Selection) -> Result<Vec<String>, Why> {
        desk::types(selection)
    }

    fn paste(
        &self,
        selection: Selection,
        wanted: &Wanted,
        largest: usize,
    ) -> Result<desk::Pasted, Why> {
        desk::paste(selection, wanted, largest)
    }

    fn clear(&self, selection: Selection) -> Result<(), Why> {
        desk::clear(selection)
    }

    fn copy(
        &self,
        selection: Selection,
        types: &[String],
        bytes: Vec<u8>,
        once: bool,
    ) -> Result<(), Why> {
        desk::copy(selection, types, bytes, once)
    }
}

/// Answer `client` when the kernel puts it in a fence of this account,
/// and say in the journal what it asked and what came of it.
pub fn serve(mut client: UnixStream, clipboard: &impl Clipboard) {
    let line = match peer::fenced(&client) {
        Ok(fenced) => {
            let _ = client.set_read_timeout(Some(PATIENCE));
            answer(&fenced.session, &mut client, clipboard)
        }
        Err(outside) => {
            let _ = writeln!(client, "{}", Answer::No(Why::Unfenced));
            said::outside(outside)
        }
    };
    eprintln!("{line}");
}

/// Answer a fenced client of `session`, giving the line the journal keeps.
pub fn answer(
    session: &str,
    client: &mut (impl Read + Write),
    clipboard: &impl Clipboard,
) -> String {
    let Some(request) = request(client) else {
        let _ = writeln!(client, "{}", Answer::No(Why::Malformed));
        return said::refused(session, None, Why::Malformed);
    };
    let selection = request.selection();
    match done(session, &request, client, clipboard) {
        Ok(line) => line,
        Err(why) => {
            let _ = writeln!(client, "{}", Answer::No(why));
            said::refused(session, Some(selection), why)
        }
    }
}

/// Do what `request` asks, giving the line the journal keeps, or why not.
fn done(
    session: &str,
    request: &Request,
    client: &mut (impl Read + Write),
    clipboard: &impl Clipboard,
) -> Result<String, Why> {
    let written = |_: io::Error| Why::Malformed;
    match request {
        Request::Copy {
            selection,
            kind,
            once,
        } => {
            let mut bytes = Vec::new();
            client
                .take(LARGEST as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| Why::Malformed)?;
            if bytes.len() > LARGEST {
                return Err(Why::TooLarge);
            }
            let types = kinds::offered(kind.as_deref(), &bytes);
            let length = bytes.len();
            clipboard.copy(*selection, &types, bytes, *once)?;
            writeln!(client, "{}", Answer::Ok(None)).map_err(written)?;
            Ok(said::copied(session, *selection, &types[0], length))
        }
        Request::Paste { selection, wanted } => {
            let pasted = clipboard.paste(*selection, wanted, LARGEST)?;
            let kind = plain_kind(&pasted.kind).ok_or(Why::NoSuchType)?;
            writeln!(client, "{}", Answer::Ok(Some(kind.to_owned()))).map_err(written)?;
            client.write_all(&pasted.bytes).map_err(written)?;
            Ok(said::pasted(session, *selection, kind, pasted.bytes.len()))
        }
        Request::List { selection } => {
            let types = clipboard.types(*selection)?;
            // A type that could break a line apart is left out, so what
            // a client reads back is only ever one type a line.
            let plain: Vec<&str> = types.iter().filter_map(|kind| plain_kind(kind)).collect();
            let mut listing = format!("{}\n", Answer::Ok(None));
            for kind in &plain {
                listing.push_str(kind);
                listing.push('\n');
            }
            client.write_all(listing.as_bytes()).map_err(written)?;
            Ok(said::listed(session, *selection, plain.len()))
        }
        Request::Clear { selection } => {
            clipboard.clear(*selection)?;
            writeln!(client, "{}", Answer::Ok(None)).map_err(written)?;
            Ok(said::cleared(session, *selection))
        }
    }
}

/// The request a client's first line says, or nothing when it says none.
fn request(client: &mut impl Read) -> Option<Request> {
    let mut line = Vec::new();
    let mut byte = [0];
    while line.len() <= LONGEST_LINE {
        match client.read(&mut byte) {
            Ok(1) if byte[0] == b'\n' => return Request::read(std::str::from_utf8(&line).ok()?),
            Ok(1) => line.push(byte[0]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            _ => return None,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::io::Cursor;

    /// A selection kept in memory, as data-control would hold it.
    #[derive(Default)]
    struct Kept {
        clipboard: RefCell<Option<(Vec<String>, Vec<u8>)>>,
    }

    impl Clipboard for Kept {
        fn types(&self, _: Selection) -> Result<Vec<String>, Why> {
            let held = self.clipboard.borrow();
            held.as_ref()
                .map(|(types, _)| types.clone())
                .ok_or(Why::NothingCopied)
        }

        fn paste(
            &self,
            _: Selection,
            wanted: &Wanted,
            largest: usize,
        ) -> Result<desk::Pasted, Why> {
            let held = self.clipboard.borrow();
            let (types, bytes) = held.as_ref().ok_or(Why::NothingCopied)?;
            if kinds::secret(types) {
                return Err(Why::Secret);
            }
            let kind = kinds::chosen(types, wanted).ok_or(Why::NoSuchType)?;
            if bytes.len() > largest {
                return Err(Why::TooLarge);
            }
            Ok(desk::Pasted {
                kind: kind.to_owned(),
                bytes: bytes.clone(),
            })
        }

        fn clear(&self, _: Selection) -> Result<(), Why> {
            self.clipboard.replace(None);
            Ok(())
        }

        fn copy(&self, _: Selection, types: &[String], bytes: Vec<u8>, _: bool) -> Result<(), Why> {
            self.clipboard.replace(Some((types.to_vec(), bytes)));
            Ok(())
        }
    }

    /// A client's whole side of a conversation: what it sends, then what
    /// it's sent back.
    struct Client {
        sent: Cursor<Vec<u8>>,
        heard: Vec<u8>,
    }

    impl Read for Client {
        fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
            self.sent.read(into)
        }
    }

    impl Write for Client {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.heard.write(bytes)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn asked(kept: &Kept, sent: &[u8]) -> (String, String) {
        let mut client = Client {
            sent: Cursor::new(sent.to_vec()),
            heard: Vec::new(),
        };
        let line = answer("s1", &mut client, kept);
        (String::from_utf8_lossy(&client.heard).into_owned(), line)
    }

    #[test]
    fn a_copy_pastes_back_as_text() {
        let kept = Kept::default();
        let (heard, line) = asked(&kept, b"copy primary - always\nhello there");
        assert_eq!(heard, "ok\n");
        assert_eq!(
            line,
            "copied session=s1 selection=primary type=text/plain;charset=utf-8 bytes=11"
        );
        let (heard, line) = asked(&kept, b"paste primary -\n");
        assert_eq!(heard, "ok text/plain;charset=utf-8\nhello there");
        assert_eq!(
            line,
            "pasted session=s1 selection=primary type=text/plain;charset=utf-8 bytes=11"
        );
    }

    #[test]
    fn a_secret_is_never_read() {
        let kept = Kept::default();
        kept.clipboard.replace(Some((
            vec!["text/plain".into(), "x-kde-passwordManagerHint".into()],
            b"hunter2".to_vec(),
        )));
        let (heard, line) = asked(&kept, b"paste clipboard -\n");
        assert_eq!(heard, "no secret\n");
        assert_eq!(line, "refused session=s1 selection=clipboard why=secret");
    }

    #[test]
    fn a_listing_leaves_out_a_type_that_would_break_its_lines() {
        let kept = Kept::default();
        kept.clipboard.replace(Some((
            vec!["text/plain".into(), "evil\nok".into(), "image/png".into()],
            Vec::new(),
        )));
        let (heard, line) = asked(&kept, b"list clipboard\n");
        assert_eq!(heard, "ok\ntext/plain\nimage/png\n");
        assert_eq!(line, "listed session=s1 selection=clipboard types=2");
    }

    #[test]
    fn a_cleared_selection_has_nothing_to_paste() {
        let kept = Kept::default();
        asked(&kept, b"copy clipboard text/html always\n<b>");
        let (heard, line) = asked(&kept, b"clear clipboard\n");
        assert_eq!(
            (heard.as_str(), line.as_str()),
            ("ok\n", "cleared session=s1 selection=clipboard")
        );
        let (heard, line) = asked(&kept, b"paste clipboard -\n");
        assert_eq!(heard, "no nothing-copied\n");
        assert_eq!(
            line,
            "refused session=s1 selection=clipboard why=nothing-copied"
        );
    }

    #[test]
    fn a_type_not_offered_is_refused() {
        let kept = Kept::default();
        asked(&kept, b"copy clipboard - always\ntext");
        let (heard, _) = asked(&kept, b"paste clipboard image/png\n");
        assert_eq!(heard, "no no-such-type\n");
    }

    #[test]
    fn a_request_unread_is_malformed() {
        let kept = Kept::default();
        for sent in [
            &b"copy clipboard"[..],
            b"take clipboard\n",
            b"\xff\n",
            &[b'a'; 2000],
        ] {
            let (heard, line) = asked(&kept, sent);
            assert_eq!(heard, "no malformed\n");
            assert_eq!(line, "refused session=s1 selection=- why=malformed");
        }
    }

    #[test]
    fn more_than_it_carries_is_too_large() {
        let kept = Kept::default();
        let mut sent = b"copy clipboard - always\n".to_vec();
        sent.resize(sent.len() + LARGEST + 1, b'a');
        let (heard, line) = asked(&kept, &sent);
        assert_eq!(heard, "no too-large\n");
        assert_eq!(line, "refused session=s1 selection=clipboard why=too-large");
        assert!(
            kept.clipboard.borrow().is_none(),
            "a copy too large was kept"
        );
    }

    #[test]
    fn a_client_outside_the_fence_is_refused_before_it_asks() {
        let (ours, mut theirs) = UnixStream::pair().unwrap();
        serve(ours, &Kept::default());
        let mut heard = String::new();
        theirs.read_to_string(&mut heard).unwrap();
        assert_eq!(heard, "no unfenced\n");
    }
}
