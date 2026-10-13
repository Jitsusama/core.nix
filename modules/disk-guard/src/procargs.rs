//! A process's arguments and environment as macOS gives them, through
//! `sysctl` and `KERN_PROCARGS2`. Read on any platform, so Linux's tests
//! hold it.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Args {
    pub argv: Vec<OsString>,
    /// The variables `keep` chose, by name and value.
    pub env: Vec<(OsString, OsString)>,
}

/// Reads the buffer: the argument count, the executable's path and the
/// NULs padding it, that many arguments, then the environment up to an
/// empty string, after which come the loader's own strings. A buffer
/// that ends before its count of arguments does is refused.
pub fn parse(bytes: &[u8], keep: impl Fn(&[u8]) -> bool) -> Option<Args> {
    let argc = i32::from_ne_bytes(bytes.get(..4)?.try_into().ok()?);
    let argc = usize::try_from(argc).ok()?;
    let mut rest = bytes.get(4..)?;
    let path = rest.iter().position(|&byte| byte == 0)?;
    rest = &rest[path..];
    while let [0, tail @ ..] = rest {
        rest = tail;
    }
    // Each string ends with a NUL; one that doesn't was cut off.
    let mut next = || {
        let end = rest.iter().position(|&byte| byte == 0)?;
        let word = &rest[..end];
        rest = &rest[end + 1..];
        Some(word)
    };
    let mut argv = Vec::with_capacity(argc);
    for _ in 0..argc {
        argv.push(OsStr::from_bytes(next()?).to_owned());
    }
    let mut env = Vec::new();
    while let Some(pair) = next().filter(|pair| !pair.is_empty()) {
        let Some(equals) = pair.iter().position(|&byte| byte == b'=') else {
            continue;
        };
        let (name, value) = (&pair[..equals], &pair[equals + 1..]);
        if keep(name) {
            env.push((
                OsStr::from_bytes(name).to_owned(),
                OsStr::from_bytes(value).to_owned(),
            ));
        }
    }
    Some(Args { argv, env })
}
