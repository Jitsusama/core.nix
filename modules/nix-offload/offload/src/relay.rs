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
//!
//! A builder near its end hands over to a successor, whose relay listens
//! on the same port beside this one's. Told to leave, a relay takes no new
//! connection, so each goes to the successor, and keeps every one it has
//! until it closes, ending with the last.
//!
//! A tunnel can go on running while every connection it opens fails. Once
//! the far side has answered no caller for a while, the relay ends with
//! its connections, so the helper's hold on the builder ends with it.

use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::os::fd::{AsRawFd, FromRawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
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
    /// How long the far side may answer no caller before the relay ends:
    /// on 5 November from 22:12 a tunnel's process lived on while every
    /// connection it opened failed, and its builder stayed lent, nothing
    /// reaching it, for half an hour. Ending ends the helper's hold, so
    /// the builder is given back and another lent.
    pub gives_up: Duration,
}

/// The longest first line waited for; ssh's is at most 255 bytes.
const LONGEST: usize = 255;

/// How often a relay looks to see whether it's been told to leave.
const HEARD_WITHIN: Duration = Duration::from_millis(100);

/// A listener on each address `name` gives that can be bound, as
/// `localhost` gives both `::1` and `127.0.0.1`, each sharing its port
/// with any other relay's, as a successor's does while this one leaves.
pub fn listening(name: &str) -> Vec<TcpListener> {
    name.to_socket_addrs()
        .map(|addresses| {
            addresses
                .filter_map(|address| shared(address).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// How long a socket option or address of type `T` is, as the kernel is
/// told it.
fn length_of<T>() -> libc::socklen_t {
    libc::socklen_t::try_from(size_of::<T>()).expect("a socket option or address fits")
}

/// An address family as a socket address names it.
fn family_of(family: libc::c_int) -> libc::sa_family_t {
    libc::sa_family_t::try_from(family).expect("an address family fits")
}

/// A listener on `address` that another may bind beside
/// (`SO_REUSEPORT`), which std's can't.
fn shared(address: SocketAddr) -> io::Result<TcpListener> {
    let failed = |result: libc::c_int| {
        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    };
    let family = match address {
        SocketAddr::V4(_) => libc::AF_INET,
        SocketAddr::V6(_) => libc::AF_INET6,
    };
    // SAFETY: a new socket, owned by the listener from here, which closes
    // it on any failure below.
    let fd = unsafe { libc::socket(family, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0) };
    failed(fd)?;
    let listener = unsafe { TcpListener::from_raw_fd(fd) };
    let set = |level, option| {
        let on: libc::c_int = 1;
        // SAFETY: an int option on a socket this owns.
        failed(unsafe {
            libc::setsockopt(
                fd,
                level,
                option,
                (&raw const on).cast(),
                length_of::<libc::c_int>(),
            )
        })
    };
    set(libc::SOL_SOCKET, libc::SO_REUSEADDR)?;
    set(libc::SOL_SOCKET, libc::SO_REUSEPORT)?;
    // SAFETY: each address is written whole before it's bound, and bind
    // reads only the length given.
    let bound = match address {
        SocketAddr::V4(v4) => {
            let mut raw: libc::sockaddr_in = unsafe { std::mem::zeroed() };
            raw.sin_family = family_of(libc::AF_INET);
            raw.sin_port = v4.port().to_be();
            raw.sin_addr.s_addr = u32::from(*v4.ip()).to_be();
            unsafe {
                libc::bind(
                    fd,
                    (&raw const raw).cast(),
                    length_of::<libc::sockaddr_in>(),
                )
            }
        }
        SocketAddr::V6(v6) => {
            // Its own family alone, so ::1 never takes 127.0.0.1's.
            set(libc::IPPROTO_IPV6, libc::IPV6_V6ONLY)?;
            let mut raw: libc::sockaddr_in6 = unsafe { std::mem::zeroed() };
            raw.sin6_family = family_of(libc::AF_INET6);
            raw.sin6_port = v6.port().to_be();
            raw.sin6_addr.s6_addr = v6.ip().octets();
            raw.sin6_scope_id = v6.scope_id();
            unsafe {
                libc::bind(
                    fd,
                    (&raw const raw).cast(),
                    length_of::<libc::sockaddr_in6>(),
                )
            }
        }
    };
    failed(bound)?;
    // SAFETY: a bound socket this owns.
    failed(unsafe { libc::listen(fd, 128) })?;
    listener.set_nonblocking(true)?;
    Ok(listener)
}

impl Relay {
    /// Passes on every connection `listeners` take, each in a thread of its
    /// own, until `leaving` is set; then takes those already waiting,
    /// listens no more, and returns once every one taken has closed.
    /// Whether it gave up instead, the far side answering no caller for
    /// [`Relay::gives_up`].
    pub fn serve_all(&self, listeners: Vec<TcpListener>, leaving: &AtomicBool) -> bool {
        let mut polled: Vec<libc::pollfd> = listeners
            .iter()
            .map(|listener| libc::pollfd {
                fd: listener.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            })
            .collect();
        let within = libc::c_int::try_from(HEARD_WITHIN.as_millis()).unwrap_or(100);
        let mut passing = Vec::new();
        let failing = Failing::default();
        while !leaving.load(Ordering::SeqCst) {
            // SAFETY: polls the listeners' own sockets, which outlive it.
            unsafe { libc::poll(polled.as_mut_ptr(), polled.len() as libc::nfds_t, within) };
            self.take(&listeners, &mut passing, &failing);
            passing.retain(|pass: &JoinHandle<()>| !pass.is_finished());
            if failing.for_at_least(self.gives_up) {
                // Its connections go with it: none reaches the far side.
                eprintln!(
                    "nix-offload: {} has answered no caller for {} s, so the relay ends, and the \
                     builder behind it goes",
                    self.to,
                    self.gives_up.as_secs()
                );
                return true;
            }
        }
        // Those already waiting are taken before the listeners close, which
        // would refuse them.
        self.take(&listeners, &mut passing, &failing);
        drop(listeners);
        eprintln!(
            "nix-offload: told to leave, so the relay takes no new connection, and ends once \
             its {} open close",
            passing.len()
        );
        for pass in passing {
            let _ = pass.join();
        }
        false
    }

    /// Takes every connection waiting on `listeners`, passing each on in a
    /// thread of its own.
    fn take(
        &self,
        listeners: &[TcpListener],
        passing: &mut Vec<JoinHandle<()>>,
        failing: &Failing,
    ) {
        for listener in listeners {
            loop {
                match listener.accept() {
                    Ok((caller, _)) => {
                        if caller.set_nonblocking(false).is_err() {
                            continue;
                        }
                        let (relay, failing) = (self.clone(), failing.clone());
                        passing.push(thread::spawn(move || relay.pass(caller, &failing)));
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                    // Out of files, say: waited on rather than polled for
                    // again at once.
                    Err(_) => {
                        thread::sleep(HEARD_WITHIN);
                        break;
                    }
                }
            }
        }
    }

    /// Passes one caller on, or hangs up on it when the far side never says
    /// its first line in time, saying which to `failing`.
    fn pass(&self, mut caller: TcpStream, failing: &Failing) {
        let Some((far, first)) = self.reached() else {
            eprintln!(
                "nix-offload: {} never answered within {} s, so a caller was hung up on",
                self.to,
                self.within.as_secs()
            );
            failing.failed();
            return;
        };
        failing.answered();
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

/// Since when every caller has been hung up on, shared by the threads
/// passing them: none while the last caller was answered, or none came.
#[derive(Clone, Default)]
struct Failing(Arc<Mutex<Option<Instant>>>);

impl Failing {
    fn failed(&self) {
        if let Ok(mut since) = self.0.lock() {
            since.get_or_insert_with(Instant::now);
        }
    }

    fn answered(&self) {
        if let Ok(mut since) = self.0.lock() {
            *since = None;
        }
    }

    fn for_at_least(&self, long: Duration) -> bool {
        self.0
            .lock()
            .is_ok_and(|since| since.is_some_and(|since| since.elapsed() >= long))
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

    fn relay_to(far: &TcpListener, within: Duration) -> Relay {
        Relay {
            to: far.local_addr().expect("the far address").to_string(),
            within,
            pause: Duration::from_millis(10),
            gives_up: Duration::from_secs(60),
        }
    }

    /// A relay to `far` on a port of its own, told to leave once `leaving`
    /// is set, ending its thread once its last connection closes.
    fn serving(far: &TcpListener, at: &str, leaving: &'static AtomicBool) -> JoinHandle<()> {
        let listeners = listening(at);
        assert!(!listeners.is_empty(), "nothing listened on {at}");
        let relay = relay_to(far, Duration::from_secs(5));
        thread::spawn(move || {
            relay.serve_all(listeners, leaving);
        })
    }

    fn relayed(far: &TcpListener, within: Duration) -> TcpStream {
        static NEVER: AtomicBool = AtomicBool::new(false);
        let listeners = listening("127.0.0.1:0");
        let at = listeners[0].local_addr().expect("its address");
        let relay = relay_to(far, within);
        thread::spawn(move || relay.serve_all(listeners, &NEVER));
        TcpStream::connect(at).expect("the relay")
    }

    /// A port nothing listens on now.
    fn free_port() -> u16 {
        let held = TcpListener::bind("127.0.0.1:0").expect("a port");
        held.local_addr().expect("its address").port()
    }

    #[test]
    fn a_successors_relay_listens_on_the_port_beside_the_leaving_ones() {
        let at = format!("127.0.0.1:{}", free_port());
        let first = listening(&at);
        assert_eq!(first.len(), 1);
        assert_eq!(listening(&at).len(), 1, "a second couldn't bind beside it");
    }

    #[test]
    fn a_relay_told_to_leave_takes_no_new_connection_and_ends_once_its_last_closes() {
        static LEAVING: AtomicBool = AtomicBool::new(false);
        static STAYING: AtomicBool = AtomicBool::new(false);
        let (old_far, new_far) = (
            TcpListener::bind("127.0.0.1:0").expect("a port"),
            TcpListener::bind("127.0.0.1:0").expect("a port"),
        );
        let at = format!("127.0.0.1:{}", free_port());
        let leaving = serving(&old_far, &at, &LEAVING);
        let mut open = TcpStream::connect(&at).expect("the relay");
        let mut far = dialled(&old_far);
        far.write_all(b"SSH-2.0-old\r\n").expect("its line");
        let mut line = [0; 13];
        open.read_exact(&mut line).expect("the old builder's line");
        let _successor = serving(&new_far, &at, &STAYING);
        LEAVING.store(true, Ordering::SeqCst);
        thread::sleep(HEARD_WITHIN * 3);
        // Every new connection goes to the successor's builder.
        for _ in 0..4 {
            let _caller = TcpStream::connect(&at).expect("the successor's relay");
            drop(dialled(&new_far));
        }
        old_far.set_nonblocking(true).expect("non-blocking");
        assert!(old_far.accept().is_err(), "the leaving relay took one");
        // The open one goes on.
        open.write_all(b"still\n").expect("a word up");
        let mut heard = [0; 6];
        far.read_exact(&mut heard).expect("the word");
        assert_eq!(&heard, b"still\n");
        assert!(!leaving.is_finished(), "it ended with a connection open");
        drop(open);
        drop(far);
        let ending = Instant::now() + Duration::from_secs(5);
        while !leaving.is_finished() {
            assert!(
                Instant::now() < ending,
                "it never ended once its last closed"
            );
            thread::sleep(Duration::from_millis(10));
        }
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
    fn a_relay_whose_far_side_answers_no_caller_for_long_enough_ends() {
        static NEVER: AtomicBool = AtomicBool::new(false);
        let relay = |gives_up| Relay {
            to: format!("127.0.0.1:{}", free_port()),
            within: Duration::from_millis(50),
            pause: Duration::from_millis(10),
            gives_up,
        };
        let listeners = listening("127.0.0.1:0");
        let at = listeners[0].local_addr().expect("its address");
        let failing = relay(Duration::from_millis(400));
        let serving = thread::spawn(move || failing.serve_all(listeners, &NEVER));
        let ending = Instant::now() + Duration::from_secs(5);
        while !serving.is_finished() {
            assert!(
                Instant::now() < ending,
                "the relay kept on with nothing answering"
            );
            if let Ok(mut caller) = TcpStream::connect(at) {
                let _ = caller.read_to_end(&mut Vec::new());
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert!(serving.join().expect("the relay"), "it says it gave up");

        // Idle, with no caller to fail, it keeps on.
        let listeners = listening("127.0.0.1:0");
        let idle = relay(Duration::from_millis(100));
        let serving = thread::spawn(move || idle.serve_all(listeners, &NEVER));
        thread::sleep(Duration::from_millis(500));
        assert!(!serving.is_finished(), "an idle relay ended");
    }

    #[test]
    fn a_far_side_that_never_speaks_in_time_is_hung_up_on_once() {
        let far = TcpListener::bind("127.0.0.1:0").expect("a port");
        let mut caller = relayed(&far, Duration::from_millis(300));
        let _silent = dialled(&far);
        let mut heard = Vec::new();
        caller.read_to_end(&mut heard).expect("the hang-up");
        assert_eq!(heard, [] as [u8; 0]);
        assert!(far.accept().is_err(), "a slow far side isn't dialled again");
    }
}
