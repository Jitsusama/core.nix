//! The house's key, and the mac a report is signed with, so the shaped
//! machine takes a report only from a machine holding the key
//! (modules/shaper/src/house.rs checks it). Without one, any device on
//! the line could forge a report from this machine's address.
//!
//! A signed report is `"<bytes> <ms> <unix_ms> <mac>"`: `unix_ms` is this
//! machine's wall clock, which the shaper holds within 3 s of its own, so
//! a report caught off the line can't be sent again later, and `mac` is
//! the first 16 bytes of HMAC-SHA256 over the text before it, in hex.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// The fewest bytes a key may hold: a shorter one can be guessed. The key
/// is made as 32 random bytes; the shaper refuses the same.
const KEY_LEAST: usize = 16;

/// The bytes of the mac a report carries: half of SHA-256's, as many as
/// a key's least, and the shaper refuses a report carrying any other.
const MAC_BYTES: usize = 16;

/// The key read from `path`, or why it can't be had.
pub fn key(path: &Path) -> Result<Vec<u8>, String> {
    let key =
        fs::read(path).map_err(|error| format!("{} can't be read: {error}", path.display()))?;
    if key.len() < KEY_LEAST {
        return Err(format!(
            "{} holds {} bytes, under the {KEY_LEAST} a key needs",
            path.display(),
            key.len()
        ));
    }
    Ok(key)
}

/// The mac of `text` under `key`: the first `MAC_BYTES` of its
/// HMAC-SHA256, in lower-case hex.
fn mac(key: &[u8], text: &str) -> String {
    let mut hash = <Hmac<Sha256>>::new_from_slice(key).expect("any key fits an HMAC");
    hash.update(text.as_bytes());
    hash.finalize().into_bytes()[..MAC_BYTES]
        .iter()
        .fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").expect("a String takes any write");
            hex
        })
}

/// `text` signed under `key` at `unix_ms`: the clock, then the mac of
/// both.
pub fn signed(text: &str, key: &[u8], unix_ms: u128) -> String {
    let stamped = format!("{text} {unix_ms}");
    let mac = mac(key, &stamped);
    format!("{stamped} {mac}")
}

/// The wall clock in milliseconds since 1970, or nothing on a clock set
/// before it.
pub fn unix_ms() -> Option<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|since| since.as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The key the vector is made under: the bytes 0 to 31.
    fn vector_key() -> Vec<u8> {
        (0..32).collect()
    }

    #[test]
    fn a_report_is_signed_as_the_shaper_checks_it() {
        // Made with Python's hmac and openssl's dgst alike, and checked in
        // modules/shaper/src/house.rs against the same.
        assert_eq!(
            signed("1500 2000", &vector_key(), 1_763_920_000_000),
            "1500 2000 1763920000000 ad712c900326603c04326ac069041fe5",
            "the signed report isn't the text, the clock, then the mac the shaper checks"
        );
    }

    #[test]
    fn a_key_too_short_to_be_one_is_refused_and_one_long_enough_taken() {
        let folder = std::env::temp_dir().join(format!("house-report-key-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("a folder");
        let short = folder.join("short");
        fs::write(&short, [7_u8; KEY_LEAST - 1]).expect("a short key");
        let whole = folder.join("whole");
        fs::write(&whole, vector_key()).expect("a key");
        let short_read = key(&short);
        let whole_read = key(&whole);
        let missing = key(&folder.join("missing"));
        fs::remove_dir_all(&folder).expect("the folder removed");
        assert!(
            short_read
                .as_ref()
                .is_err_and(|why| why.contains("15 bytes")),
            "a 15-byte key was taken, or the refusal didn't say its length: {short_read:?}"
        );
        assert_eq!(
            whole_read,
            Ok(vector_key()),
            "a 32-byte key wasn't read whole"
        );
        assert!(
            missing.is_err_and(|why| why.contains("can't be read")),
            "a missing key wasn't said to be unreadable"
        );
    }
}
