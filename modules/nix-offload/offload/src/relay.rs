//! `nix-offload relay --listen ADDRESS --to ADDRESS`: passes each
//! connection made to it on to another address, dialling that again while
//! it closes before saying its first line.
//!
//! A tunnel that opens a fresh connection to the far side for each one
//! made to it can fail to open one, and closes it before a byte comes
//! back; every build connecting then fails with it, 75 of 88 at once on
//! 31 October. An ssh server speaks first, so until its first line comes
//! back nothing of the caller's has been sent on, and dialling again
//! loses nothing. Once that line has come, the connection is the caller's,
//! and is never dialled again.

use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream, ToSocketAddrs};
use std::thread;
use std::time::{Duration, Instant};

/// How a relay reaches the far side.
#[derive(Clone)]
pub struct Relay {
    /// Where each connection is passed on to.
    pub to: String,
    /// How long the far side has to say its first line, across every dial,
    /// which is less than ssh waits for a server's, 10 s as the builder's
    /// entry has it.
    pub within: Duration,
    /// How long to wait before dialling again.
    pub pause: Duration,
}

/// The longest first line waited for; ssh's is at most 255 bytes.
const LONGEST: usize = 255;

/// A listener on each address `name` gives that can be bound, as
/// `localhost` gives both `::1` and `127.0.0.1`.
pub fn listening(name: &str) -> Vec<TcpListener> {
    name.to_socket_addrs()
        .map(|addresses| {
            addresses
                .filter_map(|address| TcpListener::bind(address).ok())
                .collect()
        })
        .unwrap_or_default()
}

impl Relay {
    /// Passes on every connection each of `listeners` takes, for as long as
    /// they listen.
    pub fn serve_all(&self, listeners: Vec<TcpListener>) {
        let serving: Vec<_> = listeners
            .into_iter()
            .map(|listener| {
                let relay = self.clone();
                thread::spawn(move || relay.serve(&listener))
            })
            .collect();
        for listener in serving {
            let _ = listener.join();
        }
    }

    /// Passes on every connection `listener` takes, each in a thread of
    /// its own, for as long as it listens.
    pub fn serve(&self, listener: &TcpListener) {
        for caller in listener.incoming().flatten() {
            let relay = self.clone();
            thread::spawn(move || relay.pass(caller));
        }
    }

    /// Passes one caller on, or hangs up on it when the far side never says
    /// its first line in time.
    fn pass(&self, mut caller: TcpStream) {
        let Some((far, first)) = self.reached() else {
            eprintln!(
                "nix-offload: {} never answered within {} s, so a caller was hung up on",
                self.to,
                self.within.as_secs()
            );
            return;
        };
        if caller.write_all(&first).is_err() {
            return;
        }
        let (Ok(mut far_in), Ok(mut caller_in)) = (far.try_clone(), caller.try_clone()) else {
            return;
        };
        let up = thread::spawn(move || {
            let _ = io::copy(&mut caller_in, &mut far_in);
            let _ = far_in.shutdown(Shutdown::Write);
        });
        let mut far = far;
        let _ = io::copy(&mut far, &mut caller);
        let _ = caller.shutdown(Shutdown::Write);
        let _ = up.join();
    }

    /// The far side, dialled until it says its first line, with that line;
    /// none when the time runs out first.
    fn reached(&self) -> Option<(TcpStream, Vec<u8>)> {
        let ending = Instant::now() + self.within;
        let mut dials = 0;
        loop {
            let left = ending.checked_duration_since(Instant::now())?;
            dials += 1;
            if let Ok(far) = TcpStream::connect(&self.to) {
                match first_line(&far, left) {
                    Said::Line(line) => {
                        if dials > 1 {
                            eprintln!("nix-offload: {} answered on dial {dials}", self.to);
                        }
                        return Some((far, line));
                    }
                    // A slow answer isn't a failed one: dialling again would
                    // only start the wait over.
                    Said::Nothing => return None,
                    Said::Closed => {}
                }
            }
            thread::sleep(
                self.pause
                    .min(ending.saturating_duration_since(Instant::now())),
            );
        }
    }
}

/// What the far side said first.
enum Said {
    Line(Vec<u8>),
    /// It closed, or failed, before a whole line.
    Closed,
    /// It said nothing in time.
    Nothing,
}

fn first_line(mut far: &TcpStream, within: Duration) -> Said {
    if far
        .set_read_timeout(Some(within.max(Duration::from_millis(1))))
        .is_err()
    {
        return Said::Closed;
    }
    let mut line = Vec::new();
    let mut byte = [0];
    while line.len() < LONGEST {
        match far.read(&mut byte) {
            Ok(0) => return Said::Closed,
            Ok(_) => {
                line.push(byte[0]);
                if byte[0] == b'\n' {
                    let _ = far.set_read_timeout(None);
                    return Said::Line(line);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Said::Nothing;
            }
            Err(_) => return Said::Closed,
        }
    }
    let _ = far.set_read_timeout(None);
    Said::Line(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufRead;

    /// The next connection `far` takes, failing the test when none comes
    /// within 5 s rather than waiting forever.
    fn dialled(far: &TcpListener) -> TcpStream {
        far.set_nonblocking(true).expect("non-blocking");
        let ending = Instant::now() + Duration::from_secs(5);
        loop {
            match far.accept() {
                Ok((stream, _)) => {
                    stream.set_nonblocking(false).expect("blocking");
                    return stream;
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < ending, "the relay never dialled");
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("accepting: {error}"),
            }
        }
    }

    fn relayed(far: &TcpListener, within: Duration) -> TcpStream {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
        let at = listener.local_addr().expect("its address");
        let relay = Relay {
            to: far.local_addr().expect("the far address").to_string(),
            within,
            pause: Duration::from_millis(10),
        };
        thread::spawn(move || relay.serve(&listener));
        TcpStream::connect(at).expect("the relay")
    }

    #[test]
    fn a_far_side_that_closes_before_speaking_is_dialled_again() {
        let far = TcpListener::bind("127.0.0.1:0").expect("a port");
        let caller = relayed(&far, Duration::from_secs(5));
        // The first two close at once, as a tunnel that couldn't open
        // does; the third answers as an ssh server.
        drop(dialled(&far));
        drop(dialled(&far));
        let mut third = dialled(&far);
        third.write_all(b"SSH-2.0-far\r\n").expect("its line");
        let mut reading = io::BufReader::new(caller.try_clone().expect("a clone"));
        let mut line = String::new();
        reading.read_line(&mut line).expect("the line");
        assert_eq!(line, "SSH-2.0-far\r\n");
        (&caller).write_all(b"hello\n").expect("a word up");
        let mut heard = [0; 6];
        third.read_exact(&mut heard).expect("the word");
        assert_eq!(&heard, b"hello\n");
        third.write_all(b"world\n").expect("a word down");
        line.clear();
        reading.read_line(&mut line).expect("the word back");
        assert_eq!(line, "world\n");
    }

    #[test]
    fn a_name_is_listened_on_at_every_address_it_gives() {
        let bound = listening("localhost:0");
        assert!(!bound.is_empty());
        let addresses: Vec<_> = "localhost:0"
            .to_socket_addrs()
            .expect("localhost")
            .collect();
        assert!(bound.len() <= addresses.len());
        assert!(listening("no.such.name.invalid:0").is_empty());
    }

    #[test]
    fn a_far_side_that_never_speaks_in_time_is_hung_up_on_once() {
        let far = TcpListener::bind("127.0.0.1:0").expect("a port");
        let mut caller = relayed(&far, Duration::from_millis(300));
        let _silent = dialled(&far);
        let mut heard = Vec::new();
        caller.read_to_end(&mut heard).expect("the hang-up");
        assert!(heard.is_empty());
        assert!(far.accept().is_err(), "a slow far side isn't dialled again");
    }
}
