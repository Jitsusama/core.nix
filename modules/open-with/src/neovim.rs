//! Asking a running Neovim what it holds and bringing one of its buffers
//! forward, over its own msgpack-RPC server: the one every Neovim starts
//! as it starts, at `v:servername`, which it tells its kitty window
//! (../../neovim/showing.lua). That server answers Neovim's whole API, so
//! nothing has to be loaded into Neovim first. neovim-pi's socket is the
//! same kind of server, but it starts only once its plugin loads, lives
//! where its own settings say, and is there for pi to pair with, so it's
//! left to pi.
//!
//! Each question is one `nvim_exec_lua` request, its Lua kept beside this
//! file and sent whole: what a Neovim holds (neovim-held.lua) and going
//! to one buffer (neovim-go.lua). Only as much of msgpack is written as a
//! request needs, and all of it is read, since Neovim answers with its own
//! extension types for buffers, windows and tabs.

use std::io::{self, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde_json::{Map, Number, Value};

/// What a Neovim holds: its process, folder, tabs and the files its
/// listed buffers hold (neovim-held.lua).
const HELD: &str = include_str!("neovim-held.lua");

/// Brings one buffer forward (neovim-go.lua).
const GO: &str = include_str!("neovim-go.lua");

/// How long a Neovim may take to answer before it's passed over, so one
/// stuck at a prompt never holds the launcher.
const ANSWER: Duration = Duration::from_secs(2);

/// The most a string, an array or a map may claim to hold, so a stream
/// that isn't msgpack can't have it allocate gigabytes.
const MOST: usize = 1 << 24;

/// msgpack-RPC's kinds of message.
const REQUEST: u64 = 0;
const RESPONSE: u64 = 1;

/// What one Neovim holds, as neovim-held.lua says it: `pid`, `folder`,
/// `tab` and `tabs`, `current` and `files`, each file with its `buffer`,
/// `path`, `modified`, the `tabs` showing it and when it was last used
/// (`at`, in seconds since 1970).
///
/// # Errors
///
/// When the Neovim can't be reached, takes too long, or refuses.
pub fn held(server: &Path) -> io::Result<Value> {
    ask(server, HELD, &[])
}

/// Brings a buffer forward in the Neovim at `server`, answering whether
/// it was still there.
///
/// # Errors
///
/// When the Neovim can't be reached, takes too long, or refuses.
pub fn go(server: &Path, buffer: u64) -> io::Result<bool> {
    Ok(ask(server, GO, &[buffer])? == Value::Bool(true))
}

/// Runs a Lua chunk in the Neovim at `server`, given whole numbers as its
/// arguments (`...`), and gives back what it returns.
///
/// # Errors
///
/// When the Neovim can't be reached, takes too long, or refuses.
pub fn ask(server: &Path, lua: &str, arguments: &[u64]) -> io::Result<Value> {
    let stream = UnixStream::connect(server)?;
    stream.set_read_timeout(Some(ANSWER))?;
    stream.set_write_timeout(Some(ANSWER))?;
    exchange(stream, lua, arguments)
}

/// Sends one `nvim_exec_lua` request on a stream and reads until its
/// response comes, passing over any notification before it.
///
/// # Errors
///
/// When the stream fails, closes, or answers with an error.
pub fn exchange(mut stream: impl Read + Write, lua: &str, arguments: &[u64]) -> io::Result<Value> {
    stream.write_all(&request(1, lua, arguments))?;
    let mut reader = BufReader::new(stream);
    loop {
        let message = read(&mut reader)?;
        let Some([kind, id, error, result]) = message.as_array().map(Vec::as_slice) else {
            continue;
        };
        if kind.as_u64() != Some(RESPONSE) || id.as_u64() != Some(1) {
            continue;
        }
        if !error.is_null() {
            let said = error[1]
                .as_str()
                .map_or_else(|| error.to_string(), str::to_owned);
            return Err(io::Error::other(said));
        }
        return Ok(result.clone());
    }
}

/// An `nvim_exec_lua` request as msgpack: `[0, id, method, [lua, args]]`.
#[must_use]
pub fn request(id: u64, lua: &str, arguments: &[u64]) -> Vec<u8> {
    let mut out = Vec::new();
    array(&mut out, 4);
    unsigned(&mut out, REQUEST);
    unsigned(&mut out, id);
    string(&mut out, "nvim_exec_lua");
    array(&mut out, 2);
    string(&mut out, lua);
    array(&mut out, arguments.len());
    for &argument in arguments {
        unsigned(&mut out, argument);
    }
    out
}

/// A length as msgpack writes it after its marker, in the fewest bytes
/// its three widths allow.
fn sized(out: &mut Vec<u8>, length: usize, markers: [u8; 3]) {
    if let Ok(short) = u8::try_from(length)
        && markers[0] != 0
    {
        out.extend([markers[0], short]);
    } else if let Ok(middle) = u16::try_from(length) {
        out.push(markers[1]);
        out.extend(middle.to_be_bytes());
    } else {
        out.push(markers[2]);
        // Nothing sent is past four gigabytes.
        out.extend(u32::try_from(length).unwrap_or(u32::MAX).to_be_bytes());
    }
}

fn array(out: &mut Vec<u8>, length: usize) {
    match u8::try_from(length) {
        Ok(small) if small < 16 => out.push(0x90 | small),
        _ => sized(out, length, [0, 0xdc, 0xdd]),
    }
}

fn string(out: &mut Vec<u8>, text: &str) {
    match u8::try_from(text.len()) {
        Ok(small) if small < 32 => out.push(0xa0 | small),
        _ => sized(out, text.len(), [0xd9, 0xda, 0xdb]),
    }
    out.extend(text.as_bytes());
}

fn unsigned(out: &mut Vec<u8>, number: u64) {
    if let Ok(small) = u8::try_from(number)
        && small < 0x80
    {
        out.push(small);
    } else if let Ok(small) = u32::try_from(number) {
        out.push(0xce);
        out.extend(small.to_be_bytes());
    } else {
        out.push(0xcf);
        out.extend(number.to_be_bytes());
    }
}

fn bytes<const N: usize>(from: &mut impl Read) -> io::Result<[u8; N]> {
    let mut read = [0; N];
    from.read_exact(&mut read)?;
    Ok(read)
}

fn length(from: &mut impl Read, width: usize) -> io::Result<usize> {
    let length = match width {
        1 => usize::from(bytes::<1>(from)?[0]),
        2 => usize::from(u16::from_be_bytes(bytes(from)?)),
        _ => usize::try_from(u32::from_be_bytes(bytes(from)?)).map_err(io::Error::other)?,
    };
    if length > MOST {
        return Err(io::Error::other("a msgpack value claims to hold too much"));
    }
    Ok(length)
}

fn raw(from: &mut impl Read, length: usize) -> io::Result<Vec<u8>> {
    let mut read = vec![0; length];
    from.read_exact(&mut read)?;
    Ok(read)
}

fn text(from: &mut impl Read, length: usize) -> io::Result<Value> {
    Ok(Value::String(
        String::from_utf8_lossy(&raw(from, length)?).into_owned(),
    ))
}

fn list(from: &mut impl Read, length: usize) -> io::Result<Value> {
    (0..length)
        .map(|_| read(from))
        .collect::<io::Result<_>>()
        .map(Value::Array)
}

fn map(from: &mut impl Read, length: usize) -> io::Result<Value> {
    let mut held = Map::new();
    for _ in 0..length {
        let key = match read(from)? {
            Value::String(key) => key,
            other => other.to_string(),
        };
        held.insert(key, read(from)?);
    }
    Ok(Value::Object(held))
}

/// An extension: Neovim's buffer, window and tab handles are each a
/// msgpack number inside one, given back as that number.
fn extension(from: &mut impl Read, length: usize) -> io::Result<Value> {
    let _kind = bytes::<1>(from)?;
    let data = raw(from, length)?;
    Ok(read(&mut data.as_slice()).unwrap_or(Value::Null))
}

fn float(number: f64) -> Value {
    Number::from_f64(number).map_or(Value::Null, Value::Number)
}

/// One msgpack value, read as JSON's nearest: a binary string as text,
/// a map's keys as text, and an extension as what it holds.
///
/// # Errors
///
/// When the stream fails or ends inside a value.
pub fn read(from: &mut impl Read) -> io::Result<Value> {
    let marker = bytes::<1>(from)?[0];
    match marker {
        0x00..=0x7f => Ok(Value::from(marker)),
        0x80..=0x8f => map(from, usize::from(marker & 0x0f)),
        0x90..=0x9f => list(from, usize::from(marker & 0x0f)),
        0xa0..=0xbf => text(from, usize::from(marker & 0x1f)),
        0xc0 => Ok(Value::Null),
        0xc2 => Ok(Value::Bool(false)),
        0xc3 => Ok(Value::Bool(true)),
        0xc4..=0xc6 => {
            let size = length(from, 1 << (marker - 0xc4))?;
            text(from, size)
        }
        0xc7..=0xc9 => {
            let size = length(from, 1 << (marker - 0xc7))?;
            extension(from, size)
        }
        0xca => Ok(float(f64::from(f32::from_be_bytes(bytes(from)?)))),
        0xcb => Ok(float(f64::from_be_bytes(bytes(from)?))),
        0xcc => Ok(Value::from(u8::from_be_bytes(bytes(from)?))),
        0xcd => Ok(Value::from(u16::from_be_bytes(bytes(from)?))),
        0xce => Ok(Value::from(u32::from_be_bytes(bytes(from)?))),
        0xcf => Ok(Value::from(u64::from_be_bytes(bytes(from)?))),
        0xd0 => Ok(Value::from(i8::from_be_bytes(bytes(from)?))),
        0xd1 => Ok(Value::from(i16::from_be_bytes(bytes(from)?))),
        0xd2 => Ok(Value::from(i32::from_be_bytes(bytes(from)?))),
        0xd3 => Ok(Value::from(i64::from_be_bytes(bytes(from)?))),
        0xd4..=0xd8 => extension(from, 1 << (marker - 0xd4)),
        0xd9..=0xdb => {
            let size = length(from, 1 << (marker - 0xd9))?;
            text(from, size)
        }
        0xdc | 0xdd => {
            let size = length(from, if marker == 0xdc { 2 } else { 4 })?;
            list(from, size)
        }
        0xde | 0xdf => {
            let size = length(from, if marker == 0xde { 2 } else { 4 })?;
            map(from, size)
        }
        0xe0..=0xff => Ok(Value::from(i8::from_be_bytes([marker]))),
        _ => Err(io::Error::other(format!(
            "msgpack has no marker {marker:#x}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::thread;
    use std::time::Instant;

    /// A stream that answers with what was recorded, keeping what it's sent.
    struct Recorded {
        answer: io::Cursor<Vec<u8>>,
        sent: Vec<u8>,
    }

    impl Read for Recorded {
        fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
            self.answer.read(into)
        }
    }

    impl Write for Recorded {
        fn write(&mut self, from: &[u8]) -> io::Result<usize> {
            self.sent.extend_from_slice(from);
            Ok(from.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn recorded(answer: &[u8]) -> Recorded {
        Recorded {
            answer: io::Cursor::new(answer.to_vec()),
            sent: Vec::new(),
        }
    }

    #[test]
    fn a_request_is_written_as_neovim_reads_it() {
        assert_eq!(
            request(1, "return 1", &[7]),
            [
                [0x94, 0x00, 0x01, 0xad].as_slice(),
                b"nvim_exec_lua",
                &[0x92, 0xa8],
                b"return 1",
                &[0x91, 0x07],
            ]
            .concat()
        );
        let long = "x".repeat(300);
        let written = request(2, &long, &[300, 1 << 40]);
        // The method ends at 17, and the Lua after its array's marker
        // is long enough for two bytes of length.
        assert_eq!(&written[17..21], &[0x92, 0xda, 0x01, 0x2c]);
        assert_eq!(
            &written[321..],
            &[
                0x92, 0xce, 0x00, 0x00, 0x01, 0x2c, 0xcf, 0, 0, 1, 0, 0, 0, 0, 0
            ]
        );
    }

    #[test]
    fn every_kind_of_value_is_read() {
        let value = |bytes: &[u8]| read(&mut &bytes[..]).unwrap();
        assert_eq!(value(&[0x05]), json!(5));
        assert_eq!(value(&[0xff]), json!(-1));
        assert_eq!(value(&[0xd0, 0x80]), json!(-128));
        assert_eq!(value(&[0xcd, 0x01, 0x00]), json!(256));
        assert_eq!(value(&[0xc0]), Value::Null);
        assert_eq!(value(&[0xc3]), json!(true));
        assert_eq!(value(&[0xa2, b'h', b'i']), json!("hi"));
        assert_eq!(value(&[0xd9, 0x01, b'a']), json!("a"));
        assert_eq!(value(&[0x92, 0x01, 0xc2]), json!([1, false]));
        assert_eq!(value(&[0x81, 0xa1, b'k', 0x02]), json!({ "k": 2 }));
        assert_eq!(value(&[0xcb, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0]), json!(1.5));
        // A buffer handle, as Neovim sends one: its own extension type
        // around the buffer's number.
        assert_eq!(value(&[0xd4, 0x00, 0x03]), json!(3));
        assert_eq!(value(&[0xc7, 0x02, 0x01, 0xcc, 0xc8]), json!(200));
    }

    #[test]
    fn a_value_claiming_too_much_or_cut_short_is_refused() {
        assert!(
            read(&mut &[0xdb, 0xff, 0xff, 0xff, 0xff][..]).is_err(),
            "a string claiming four billion bytes"
        );
        assert!(
            read(&mut &[0xa3, b'a'][..]).is_err(),
            "a string of three cut after one"
        );
        assert!(read(&mut &[][..]).is_err(), "nothing at all");
    }

    #[test]
    fn the_answer_is_read_past_a_notification() {
        // A notification, then the response to request 1.
        let mut answer = vec![0x93, 0x02, 0xa4];
        answer.extend(b"note");
        answer.extend([0x90, 0x94, 0x01, 0x01, 0xc0, 0x81, 0xa3]);
        answer.extend(b"pid");
        answer.extend([0xcd, 0x10, 0x00]);
        let mut stream = recorded(&answer);
        assert_eq!(
            exchange(&mut stream, "return 1", &[]).unwrap(),
            json!({ "pid": 4096 })
        );
        assert_eq!(stream.sent, request(1, "return 1", &[]));
    }

    #[test]
    fn an_error_neovim_answers_is_said() {
        let mut answer = vec![0x94, 0x01, 0x01, 0x92, 0x01, 0xa6];
        answer.extend(b"broken");
        answer.push(0xc0);
        let error = exchange(recorded(&answer), "return", &[]).unwrap_err();
        assert_eq!(error.to_string(), "broken");
    }

    /// A headless Neovim with no config of anyone's, listening in a folder
    /// of the test's own, its state there too, stopped once dropped.
    struct Headless {
        child: Child,
        folder: PathBuf,
        server: PathBuf,
    }

    impl Drop for Headless {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
            let _ = fs::remove_dir_all(&self.folder);
        }
    }

    fn headless(name: &str, files: &[&str]) -> Headless {
        let made = std::env::temp_dir().join(format!("open-now-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&made);
        fs::create_dir_all(made.join("run")).unwrap();
        // Neovim names its files from the folder it's in, links resolved,
        // so the test does too, wherever its temporary folder links.
        let folder = fs::canonicalize(&made).unwrap();
        for file in files {
            fs::write(folder.join(file), format!("{file}\n")).unwrap();
        }
        // The server listens where the folder was made, since a socket's
        // path can't pass 108 bytes and the folder's links may be longer.
        let server = made.join("nvim.sock");
        let child = Command::new("nvim")
            .args(["--headless", "--clean", "-n", "--listen"])
            .arg(&server)
            .args(files)
            .current_dir(&folder)
            .env("HOME", &folder)
            .env("XDG_CONFIG_HOME", folder.join("config"))
            .env("XDG_DATA_HOME", folder.join("data"))
            .env("XDG_STATE_HOME", folder.join("state"))
            .env("XDG_RUNTIME_DIR", folder.join("run"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .expect("nvim, which the build's checks bring");
        let started = Instant::now();
        while UnixStream::connect(&server).is_err() {
            assert!(
                started.elapsed() < Duration::from_secs(20),
                "nvim never listened"
            );
            thread::sleep(Duration::from_millis(20));
        }
        Headless {
            child,
            folder,
            server,
        }
    }

    #[test]
    fn a_running_neovim_says_its_files_and_tabs_and_goes_to_one() {
        let neovim = headless("held", &["one.txt", "two.txt", "three.txt"]);
        let path = |file: &str| neovim.folder.join(file).display().to_string();
        // two.txt in a tab of its own and changed, three.txt hidden, and
        // help, a scratch buffer and a plugin's address beside them,
        // none of which are files.
        ask(
            &neovim.server,
            "vim.cmd('tabedit two.txt'); vim.api.nvim_buf_set_lines(0, 0, 0, false, { 'new' }); \
             vim.cmd('help'); vim.cmd('enew'); vim.cmd('badd oil:///somewhere/'); \
             vim.cmd('tabfirst')",
            &[],
        )
        .unwrap();
        let held = held(&neovim.server).unwrap();
        assert_eq!(held["pid"], json!(neovim.child.id()));
        assert_eq!(held["tab"], json!(1));
        assert_eq!(held["tabs"], json!(2));
        let files: Vec<_> = held["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|file| {
                (
                    file["path"].as_str().unwrap().to_owned(),
                    file["modified"].clone(),
                    file["tabs"].clone(),
                )
            })
            .collect();
        assert_eq!(
            files,
            [
                (path("one.txt"), json!(false), json!([1])),
                (path("two.txt"), json!(true), json!([2])),
                (path("three.txt"), json!(false), json!([])),
            ]
        );
        let buffer = |file: &str| {
            held["files"]
                .as_array()
                .unwrap()
                .iter()
                .find(|one| one["path"] == path(file))
                .and_then(|one| one["buffer"].as_u64())
                .unwrap()
        };
        let now = || {
            ask(
                &neovim.server,
                "return { vim.api.nvim_tabpage_get_number(0), vim.api.nvim_buf_get_name(0) }",
                &[],
            )
            .unwrap()
        };
        assert!(
            go(&neovim.server, buffer("two.txt")).unwrap(),
            "two.txt, in the second tab, not gone to"
        );
        assert_eq!(now(), json!([2, path("two.txt")]));
        assert!(
            go(&neovim.server, buffer("three.txt")).unwrap(),
            "three.txt, hidden, not gone to"
        );
        assert_eq!(now(), json!([2, path("three.txt")]));
        assert!(
            go(&neovim.server, buffer("one.txt")).unwrap(),
            "one.txt, in the first tab, not gone to"
        );
        assert_eq!(now(), json!([1, path("one.txt")]));
        assert!(
            !go(&neovim.server, 999).unwrap(),
            "a buffer Neovim lacks said gone to"
        );
    }
}
