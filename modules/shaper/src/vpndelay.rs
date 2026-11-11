//! A call's round trip through a VPN, as the one-way controller hears it
//! (src/oneway.rs). A VPN sends everything inside one flow of its own, and
//! a router that queues flow by flow, as the eero does, queues that flow
//! apart from the probes the recorder sends on the link, so a queue for
//! the VPN delays none of them. A STUN request sent through the VPN rides
//! inside its flow and waits in that queue, as Joel's call does.

use crate::call::{STUN, stun_answer, stun_request};
use std::collections::VecDeque;
use std::ffi::CString;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::os::fd::AsRawFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::{Duration, Instant};

/// How often a request goes, as often as the proof's call sends one, and
/// how long one goes unanswered before it's given up on.
const EVERY: Duration = Duration::from_millis(200);
const LOST: Duration = Duration::from_secs(2);
/// How far back the round trips heard are summed up: a second, as the
/// controller steps.
const WINDOW: Duration = Duration::from_secs(1);

/// The probe through the VPN being followed, and the round trips it heard.
/// Each probe answers on a channel of its own, so nothing a stopped one
/// heard is taken for the next one's.
pub struct Probe {
    device: Option<String>,
    stop: Arc<AtomicBool>,
    heard: Receiver<(Instant, f64)>,
    window: VecDeque<(Instant, f64)>,
}

impl Default for Probe {
    fn default() -> Probe {
        Probe {
            device: None,
            stop: Arc::new(AtomicBool::new(true)),
            heard: channel().1,
            window: VecDeque::new(),
        }
    }
}

impl Probe {
    /// Probes through `device` to the STUN server, or nothing: a new
    /// device stops the probe through the old one.
    pub fn follow(&mut self, device: Option<&str>) {
        self.follow_to(device, STUN);
    }

    fn follow_to(&mut self, device: Option<&str>, server: &str) {
        if self.device.as_deref() == device {
            return;
        }
        self.stop.store(true, Ordering::Relaxed);
        self.device = device.map(str::to_string);
        self.window.clear();
        let (send, heard) = channel();
        self.heard = heard;
        let Some(device) = device else {
            return;
        };
        self.stop = Arc::new(AtomicBool::new(false));
        let (device, server) = (device.to_string(), server.to_string());
        let stop = Arc::clone(&self.stop);
        thread::spawn(move || probe(&device, &server, &stop, &send));
    }

    /// The middle of the round trips heard over the last second, in ms.
    pub fn heard(&mut self, now: Instant) -> Option<f64> {
        self.window.extend(self.heard.try_iter());
        while self
            .window
            .front()
            .is_some_and(|&(at, _)| now.saturating_duration_since(at) > WINDOW)
        {
            self.window.pop_front();
        }
        crate::replay::median(self.window.iter().map(|&(_, ms)| ms))
    }
}

/// Sends a STUN request through `device` every `EVERY` until `stop`,
/// sending each round trip answered to `send`.
fn probe(device: &str, server: &str, stop: &AtomicBool, send: &Sender<(Instant, f64)>) {
    while !stop.load(Ordering::Relaxed) {
        let found = server
            .to_socket_addrs()
            .ok()
            .and_then(|mut all| all.find(SocketAddr::is_ipv4));
        let socket = found.and_then(|to| bound(device).ok().map(|s| (s, to)));
        let Some((socket, to)) = socket else {
            // A VPN coming up may not yet have its route or its names, and
            // asking again at once would only spin until it does.
            thread::sleep(Duration::from_secs(5));
            continue;
        };
        if asking(&socket, to, stop, send).is_err() {
            // The device went away under the socket: as above.
            thread::sleep(Duration::from_secs(5));
        }
    }
}

/// A UDP socket whose packets leave by `device` alone.
fn bound(device: &str) -> io::Result<UdpSocket> {
    let socket = UdpSocket::bind(("0.0.0.0", 0))?;
    let name = CString::new(device).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    let length = libc::socklen_t::try_from(name.as_bytes_with_nul().len())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    // SAFETY: the option's value is the device's NUL-terminated name, and
    // its length is that name's.
    let bound = unsafe {
        libc::setsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_BINDTODEVICE,
            name.as_ptr().cast(),
            length,
        )
    };
    if bound < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(socket)
}

/// Asks `to` through `socket` until `stop`, or until a request can't go.
fn asking(
    socket: &UdpSocket,
    to: SocketAddr,
    stop: &AtomicBool,
    send: &Sender<(Instant, f64)>,
) -> io::Result<()> {
    let mut unanswered: Vec<([u8; 12], Instant)> = Vec::new();
    let mut next = Instant::now();
    let mut seq = 0_u64;
    let mut buffer = [0_u8; 512];
    while !stop.load(Ordering::Relaxed) {
        let now = Instant::now();
        if now >= next {
            seq += 1;
            let mut id = [0_u8; 12];
            id[4..].copy_from_slice(&seq.to_be_bytes());
            socket.send_to(&stun_request(id), to)?;
            unanswered.push((id, now));
            next = now + EVERY;
        }
        unanswered.retain(|&(_, at)| now - at <= LOST);
        let wait = next
            .saturating_duration_since(Instant::now())
            .max(Duration::from_millis(1));
        socket.set_read_timeout(Some(wait))?;
        if let Ok(length) = socket.recv(&mut buffer)
            && let Some(id) = stun_answer(&buffer[..length])
            && let Some(at) = unanswered.iter().position(|&(s, _)| s == id)
        {
            let (_, when) = unanswered.remove(at);
            let answered = Instant::now();
            let ms = (answered - when).as_secs_f64() * 1_000.0;
            if send.send((answered, ms)).is_err() {
                return Ok(());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A STUN server on this machine answering every request after
    /// `delay`, at the address it answers.
    fn server(delay: Duration) -> SocketAddr {
        let socket = UdpSocket::bind(("127.0.0.1", 0)).expect("a socket");
        let at = socket.local_addr().expect("an address");
        thread::spawn(move || {
            let mut buffer = [0_u8; 512];
            while let Ok((length, from)) = socket.recv_from(&mut buffer) {
                let mut answer = buffer;
                answer[0..2].copy_from_slice(&0x0101_u16.to_be_bytes());
                // The delay a queue on the way would add.
                thread::sleep(delay);
                let _ = socket.send_to(&answer[..length.max(20)], from);
            }
        });
        at
    }

    #[test]
    fn a_round_trip_through_the_device_is_heard() {
        let at = server(Duration::from_millis(30));
        let mut probe = Probe::default();
        probe.follow_to(Some("lo"), &at.to_string());
        let begun = Instant::now();
        let mut heard = None;
        while heard.is_none() && begun.elapsed() < Duration::from_secs(5) {
            // The probe asks five times a second; this waits for its answers.
            thread::sleep(Duration::from_millis(250));
            heard = probe.heard(Instant::now());
        }
        assert!(
            heard.is_some_and(|ms| (30.0..200.0).contains(&ms)),
            "heard {heard:?} of a 30 ms round trip through lo"
        );
        probe.follow_to(None, &at.to_string());
        assert_eq!(
            probe.heard(Instant::now()),
            None,
            "still heard a probe that was stopped"
        );
    }

    #[test]
    fn what_was_heard_over_a_second_ago_is_forgotten() {
        let (send, heard) = channel();
        let mut probe = Probe {
            heard,
            ..Probe::default()
        };
        let now = Instant::now();
        for (ago, ms) in [(1_500, 900.0), (600, 40.0), (400, 30.0), (200, 20.0)] {
            let at = now.checked_sub(Duration::from_millis(ago)).expect("a past");
            send.send((at, ms)).expect("a channel");
        }
        assert_eq!(
            probe.heard(now),
            Some(30.0),
            "not the middle of the last second's round trips"
        );
    }
}
