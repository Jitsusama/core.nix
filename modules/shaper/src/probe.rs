//! Pings to distant hosts over one link, a ping process each, read line by
//! line. iputils' ping already does what a probe needs (a raw socket bound
//! to the link, an interval under a second for root, a line for a reply
//! and, with -O, one for a ping that never came back), so the shaper reads
//! it rather than speaking ICMP itself.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

/// One ping's outcome: its link, its host, and its round trip in
/// milliseconds, or None when it never came back.
#[derive(Clone, Debug, PartialEq)]
pub struct Echo {
    pub link: String,
    pub host: String,
    pub rtt_ms: Option<f64>,
}

/// What a line of ping's output says of one ping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Heard {
    /// It came back after this many milliseconds.
    Back(f64),
    /// It never came back.
    Lost,
}

impl Heard {
    fn rtt_ms(self) -> Option<f64> {
        match self {
            Heard::Back(ms) => Some(ms),
            Heard::Lost => None,
        }
    }
}

/// What one line of ping's output says, if anything.
pub fn parse(line: &str) -> Option<Heard> {
    if line.starts_with("no answer yet") {
        return Some(Heard::Lost);
    }
    let at = line.find("time=")?;
    let rest = &line[at + 5..];
    let end = rest
        .find(|c: char| c != '.' && !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok().map(Heard::Back)
}

/// The pings running for one link, stopped when dropped.
pub struct Probes {
    children: Vec<Child>,
}

impl Probes {
    pub fn start(link: &str, hosts: &[String], interval_s: f64, to: &Sender<Echo>) -> Probes {
        let mut children = Vec::new();
        for host in hosts {
            let child = Command::new("ping")
                .args(["-n", "-O", "-W", "1", "-i"])
                .arg(format!("{interval_s}"))
                .args(["-I", link, host])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn();
            let Ok(mut child) = child else {
                eprintln!("couldn't start ping to {host} over {link}");
                continue;
            };
            let stdout = child.stdout.take().expect("piped");
            let (link, host, to) = (link.to_string(), host.clone(), to.clone());
            thread::spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if let Some(heard) = parse(&line) {
                        let echo = Echo {
                            link: link.clone(),
                            host: host.clone(),
                            rtt_ms: heard.rtt_ms(),
                        };
                        if to.send(echo).is_err() {
                            break;
                        }
                    }
                }
            });
            children.push(child);
        }
        Probes { children }
    }

    /// Whether every ping still runs; one that ended (its link went away,
    /// or the host can't be routed to) means starting them again.
    pub fn running(&mut self) -> bool {
        !self.children.is_empty()
            && self
                .children
                .iter_mut()
                .all(|c| matches!(c.try_wait(), Ok(None)))
    }
}

impl Drop for Probes {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_gives_its_round_trip() {
        let line = "64 bytes from 1.1.1.1: icmp_seq=3 ttl=58 time=17.3 ms";
        assert_eq!(parse(line), Some(Heard::Back(17.3)));
        assert_eq!(
            parse("64 bytes from 10.0.0.1: icmp_seq=1 ttl=64 time=0.412 ms"),
            Some(Heard::Back(0.412))
        );
    }

    #[test]
    fn a_ping_that_never_came_back_is_lost() {
        assert_eq!(parse("no answer yet for icmp_seq=4"), Some(Heard::Lost));
    }

    #[test]
    fn other_lines_say_nothing() {
        assert_eq!(
            parse("PING 1.1.1.1 (1.1.1.1) from 192.168.4.45 enp: 56(84) bytes of data."),
            None
        );
        assert_eq!(parse(""), None);
    }
}
