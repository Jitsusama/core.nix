//! A SOCKS5 proxy (RFC 1928) on a loopback port, for the sign-in window's
//! web view alone: it asks a username and password made up for this window
//! (RFC 1929), and connects wherever the page asks through the card, after
//! its network's own DNS servers say where a name is. A VPN routing
//! everything else can't take the page's traffic, nor its names.

use std::io::{self, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::thread;

const VERSION: u8 = 5;
const USERNAME_PASSWORD: u8 = 2;
const NO_ACCEPTABLE_METHOD: u8 = 0xff;
const CONNECT: u8 = 1;

/// Where the page asks to be connected.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Address(SocketAddr),
    Name(String, u16),
}

/// What the proxy answers a request with, as RFC 1928 numbers them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Answer {
    Succeeded = 0,
    Failure = 1,
    Unreachable = 4,
    Refused = 5,
    CommandNotSupported = 7,
    AddressNotSupported = 8,
}

/// The username and password the window's web view is given, and nobody
/// else knows.
#[derive(Clone, Debug)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

/// Agrees with a client how it proves who it is, which must be by username
/// and password; says whether it can go on.
pub fn greet(stream: &mut (impl Read + Write)) -> io::Result<bool> {
    let [version, count] = read_bytes(stream)?;
    let mut methods = vec![0; usize::from(count)];
    stream.read_exact(&mut methods)?;
    let going = version == VERSION && methods.contains(&USERNAME_PASSWORD);
    let method = if going {
        USERNAME_PASSWORD
    } else {
        NO_ACCEPTABLE_METHOD
    };
    stream.write_all(&[VERSION, method])?;
    Ok(going)
}

/// Reads the client's username and password, and says whether they're
/// the window's.
pub fn authenticate(
    stream: &mut (impl Read + Write),
    credentials: &Credentials,
) -> io::Result<bool> {
    let [_version] = read_bytes(stream)?;
    let username = read_counted(stream)?;
    let password = read_counted(stream)?;
    let theirs =
        username == credentials.username.as_bytes() && password == credentials.password.as_bytes();
    stream.write_all(&[1, u8::from(!theirs)])?;
    Ok(theirs)
}

/// Reads where the client asks to be connected, or answers that the proxy
/// only connects.
pub fn request(stream: &mut (impl Read + Write)) -> io::Result<Option<Target>> {
    let [_version, command, _reserved, kind] = read_bytes(stream)?;
    let target = match kind {
        1 => Some(IpAddr::from(read_bytes::<4>(stream)?)),
        4 => Some(IpAddr::from(read_bytes::<16>(stream)?)),
        3 => None,
        _ => return answer(stream, Answer::AddressNotSupported).map(|()| None),
    };
    let target = if let Some(address) = target {
        Target::Address(SocketAddr::new(
            address,
            u16::from_be_bytes(read_bytes(stream)?),
        ))
    } else {
        let name = String::from_utf8(read_counted(stream)?)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        Target::Name(name, u16::from_be_bytes(read_bytes(stream)?))
    };
    if command != CONNECT {
        answer(stream, Answer::CommandNotSupported)?;
        return Ok(None);
    }
    Ok(Some(target))
}

/// Answers a request. The address it's bound to is nobody's business, so
/// every answer gives 0.0.0.0:0.
pub fn answer(stream: &mut impl Write, answer: Answer) -> io::Result<()> {
    stream.write_all(&[VERSION, answer as u8, 0, 1, 0, 0, 0, 0, 0, 0])
}

fn read_bytes<const N: usize>(stream: &mut impl Read) -> io::Result<[u8; N]> {
    let mut bytes = [0; N];
    stream.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// Bytes led by how many there are, as names and passwords come.
fn read_counted(stream: &mut impl Read) -> io::Result<Vec<u8>> {
    let [count] = read_bytes(stream)?;
    let mut bytes = vec![0; usize::from(count)];
    stream.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// Serves every client that connects, each on a thread of its own, with
/// `connect` making the connection a request asks for.
pub fn serve<C>(listener: &TcpListener, credentials: &Credentials, connect: C)
where
    C: Fn(&Target) -> io::Result<TcpStream> + Clone + Send + 'static,
{
    for client in listener.incoming().flatten() {
        let (credentials, connect) = (credentials.clone(), connect.clone());
        thread::spawn(move || {
            let _ = relay(client, &credentials, &connect);
        });
    }
}

fn relay<C>(mut client: TcpStream, credentials: &Credentials, connect: &C) -> io::Result<()>
where
    C: Fn(&Target) -> io::Result<TcpStream>,
{
    if !greet(&mut client)? || !authenticate(&mut client, credentials)? {
        return Ok(());
    }
    let Some(target) = request(&mut client)? else {
        return Ok(());
    };
    let server = match connect(&target) {
        Ok(server) => server,
        Err(error) => return answer(&mut client, failed(&error)),
    };
    answer(&mut client, Answer::Succeeded)?;
    let (mut from_client, mut to_server) = (client.try_clone()?, server.try_clone()?);
    let upstream = thread::spawn(move || io::copy(&mut from_client, &mut to_server));
    let (mut from_server, mut to_client) = (server, client);
    let _ = io::copy(&mut from_server, &mut to_client);
    let _ = upstream.join();
    Ok(())
}

/// What a failed connection is answered with.
fn failed(error: &io::Error) -> Answer {
    match error.kind() {
        io::ErrorKind::ConnectionRefused => Answer::Refused,
        io::ErrorKind::NotFound | io::ErrorKind::HostUnreachable => Answer::Unreachable,
        _ => Answer::Failure,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::net::Ipv6Addr;

    use super::*;

    /// A client's bytes to read, and what the proxy writes back.
    struct Conversation {
        said: Cursor<Vec<u8>>,
        answered: Vec<u8>,
    }

    impl Conversation {
        fn new(said: &[u8]) -> Conversation {
            Conversation {
                said: Cursor::new(said.to_vec()),
                answered: Vec::new(),
            }
        }
    }

    impl Read for Conversation {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.said.read(buffer)
        }
    }

    impl Write for Conversation {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.answered.write(bytes)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn window() -> Credentials {
        Credentials {
            username: "window".into(),
            password: "one-time".into(),
        }
    }

    fn proving(username: &str, password: &str) -> Vec<u8> {
        let mut said = vec![1, u8::try_from(username.len()).unwrap()];
        said.extend(username.as_bytes());
        said.push(u8::try_from(password.len()).unwrap());
        said.extend(password.as_bytes());
        said
    }

    #[test]
    fn a_client_offering_a_username_and_password_is_asked_for_them() {
        let mut client = Conversation::new(&[VERSION, 2, 0, USERNAME_PASSWORD]);

        let going = greet(&mut client).unwrap();

        assert!(going);
        assert_eq!(client.answered, [VERSION, USERNAME_PASSWORD]);
    }

    #[test]
    fn a_client_offering_no_username_and_password_is_turned_away() {
        let mut client = Conversation::new(&[VERSION, 1, 0]);

        let going = greet(&mut client).unwrap();

        assert!(!going);
        assert_eq!(client.answered, [VERSION, NO_ACCEPTABLE_METHOD]);
    }

    #[test]
    fn the_window_s_own_username_and_password_let_it_on() {
        let mut client = Conversation::new(&proving("window", "one-time"));

        let going = authenticate(&mut client, &window()).unwrap();

        assert!(going);
        assert_eq!(client.answered, [1, 0]);
    }

    #[test]
    fn anyone_else_s_are_refused() {
        let mut client = Conversation::new(&proving("window", "guessed"));

        let going = authenticate(&mut client, &window()).unwrap();

        assert!(!going);
        assert_eq!(client.answered, [1, 1]);
    }

    #[test]
    fn a_request_for_a_name_is_read_with_its_port() {
        let mut said = vec![VERSION, CONNECT, 0, 3, 14];
        said.extend(b"front-desk.inn");
        said.extend(443u16.to_be_bytes());
        let mut client = Conversation::new(&said);

        let target = request(&mut client).unwrap();

        assert_eq!(target, Some(Target::Name("front-desk.inn".into(), 443)));
    }

    #[test]
    fn a_request_for_an_address_is_read_as_one() {
        let mut v4 = Conversation::new(&[VERSION, CONNECT, 0, 1, 10, 43, 0, 1, 0, 80]);
        let mut v6 = vec![VERSION, CONNECT, 0, 4];
        v6.extend(Ipv6Addr::LOCALHOST.octets());
        v6.extend(8080u16.to_be_bytes());
        let mut v6 = Conversation::new(&v6);

        let v4 = request(&mut v4).unwrap();
        let v6 = request(&mut v6).unwrap();

        assert_eq!(v4, Some(Target::Address("10.43.0.1:80".parse().unwrap())));
        assert_eq!(v6, Some(Target::Address("[::1]:8080".parse().unwrap())));
    }

    #[test]
    fn a_request_to_do_anything_but_connect_is_refused() {
        let mut client = Conversation::new(&[VERSION, 2, 0, 1, 10, 43, 0, 1, 0, 80]);

        let target = request(&mut client).unwrap();

        assert_eq!(target, None);
        assert_eq!(
            client.answered[..2],
            [VERSION, Answer::CommandNotSupported as u8]
        );
    }

    #[test]
    fn an_answer_says_how_it_went_with_no_address_to_report() {
        let mut answered = Vec::new();

        answer(&mut answered, Answer::Unreachable).unwrap();

        assert_eq!(answered, [VERSION, 4, 0, 1, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn a_connection_the_server_refuses_is_answered_so() {
        let refused = io::Error::from(io::ErrorKind::ConnectionRefused);
        let elsewhere = io::Error::from(io::ErrorKind::TimedOut);

        assert_eq!(failed(&refused), Answer::Refused);
        assert_eq!(failed(&elsewhere), Answer::Failure);
    }
}
