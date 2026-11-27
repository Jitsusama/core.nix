//! Reading a loose object: one zlib stream of a header, `kind size\0`, and
//! the object's bytes. The header is inflated alone first, and then the
//! stream no further than the size it gives, through the bounds a packed
//! object is read within, so a stream that inflates past that size, however
//! far, costs no more than the size before it's refused, and one that ends
//! short of it, or has bytes after it, is refused too, as git refuses each.

use super::{Kind, Object, Store, Stored, inflate_ending, invalid};
use crate::hash::Oid;
use miniz_oxide::inflate::stream::{InflateState, inflate};
use miniz_oxide::{DataFormat, MZError, MZFlush, MZStatus};
use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;

/// The most a header may take, NUL and all: git's `MAX_HEADER_LEN`.
const HEADER_MOST: usize = 32;

impl Store {
    /// The object `oid` loose in this store, or None where no file holds it.
    pub(super) fn loose(&self, oid: &Oid) -> io::Result<Option<Object>> {
        let hex = oid.hex();
        let path = self.dir.join(&hex[..2]).join(&hex[2..]);
        let Some(file) = crate::regular_file::open(&path)? else {
            return Ok(None);
        };
        let (kind, size, header) = header(&file)?;
        let whole = header
            .checked_add(size)
            .ok_or_else(|| invalid("loose object's size"))?;
        let (mut data, end) = inflate_ending(&file, 0, whole, Stored::Loose)?;
        // As git's `unpack_loose_rest` refuses bytes past the stream.
        if end != file.metadata()?.len() {
            return Err(invalid("garbage at end of loose object"));
        }
        data.drain(..header);
        Ok(Some(Object { kind, data }))
    }

    /// The kind the header of `oid`'s loose copy here gives, or None where
    /// no file holds it.
    pub(super) fn loose_kind(&self, oid: &Oid) -> io::Result<Option<Kind>> {
        let hex = oid.hex();
        let path = self.dir.join(&hex[..2]).join(&hex[2..]);
        let Some(file) = crate::regular_file::open(&path)? else {
            return Ok(None);
        };
        Ok(Some(header(&file)?.0))
    }
}

/// The kind and size the header of `file`'s stream gives, and how many
/// bytes it takes, inflating no more than git reads of one.
fn header(file: &File) -> io::Result<(Kind, usize, usize)> {
    let mut state = InflateState::new_boxed(DataFormat::Zlib);
    let mut out = [0; HEADER_MOST];
    let (mut filled, mut offset) = (0, 0);
    let mut chunk = [0; 64];
    loop {
        if let Some(nul) = out[..filled].iter().position(|&byte| byte == 0) {
            let (kind, size) = parse(&out[..nul]).ok_or_else(|| invalid("loose header"))?;
            return Ok((kind, size, nul + 1));
        }
        if filled == HEADER_MOST {
            return Err(invalid("loose header"));
        }
        let got = file.read_at(&mut chunk, offset)?;
        let result = inflate(&mut state, &chunk[..got], &mut out[filled..], MZFlush::None);
        filled += result.bytes_written;
        offset += result.bytes_consumed as u64;
        match result.status {
            // A stream ended, or one going nowhere, holds no more of a
            // header than it gave.
            Ok(MZStatus::StreamEnd) => {
                if !out[..filled].contains(&0) {
                    return Err(invalid("loose header"));
                }
            }
            Ok(_) | Err(MZError::Buf) if result.bytes_consumed + result.bytes_written > 0 => {}
            Ok(_) | Err(MZError::Buf) => return Err(invalid("loose header")),
            Err(_) => return Err(invalid("loose object's stream")),
        }
    }
}

/// The kind and size in a header, `kind size` without its NUL, read as git's
/// `parse_loose_header` reads them: a size of decimal digits, no others,
/// with no zero leading any but zero itself, and none past what fits.
fn parse(header: &[u8]) -> Option<(Kind, usize)> {
    let space = header.iter().position(|&byte| byte == b' ')?;
    let kind = Kind::from_name(&header[..space])?;
    let digits = &header[space + 1..];
    if digits.is_empty()
        || !digits.iter().all(u8::is_ascii_digit)
        || (digits.len() > 1 && digits[0] == b'0')
    {
        return None;
    }
    let size = digits.iter().try_fold(0usize, |size, digit| {
        size.checked_mul(10)?.checked_add(usize::from(digit - b'0'))
    })?;
    Some((kind, size))
}

#[cfg(test)]
mod tests {
    use super::super::{Kind, Odb, starve};
    use crate::hash::Oid;

    /// What the store holding `stream` as a loose object reads of it, with
    /// its objects' memory held to `starved` bytes if given.
    fn read(name: &str, stream: &[u8], starved: Option<usize>) -> Result<(Kind, Vec<u8>), String> {
        let objects = crate::pack_index::tests::unique(name);
        let oid = Oid([0x0a; 20]);
        let folder = objects.join(&oid.hex()[..2]);
        std::fs::create_dir_all(&folder).expect("the temp folder takes a folder");
        std::fs::write(folder.join(&oid.hex()[2..]), stream).expect("the temp folder takes a file");
        starve(starved);
        let read = Odb::open(&objects).read(&oid);
        starve(None);
        std::fs::remove_dir_all(&objects).expect("the temp folder gives its folder back");
        match read {
            Ok(Some(object)) => Ok((object.kind, object.data)),
            Ok(None) => Err("missing".into()),
            Err(error) => Err(error.to_string()),
        }
    }

    /// A loose object's bytes, and what reading it gives or the refusal.
    type Case<'a> = (&'a [u8], Result<(Kind, &'a [u8]), &'a str>);

    fn deflated(raw: &[u8]) -> Vec<u8> {
        miniz_oxide::deflate::compress_to_vec_zlib(raw, 6)
    }

    #[test]
    fn a_header_is_read_as_git_reads_one() {
        // A size of more digits than a header holds, past what fits too.
        let long_size = [b"blob ".as_slice(), &[b'1'; 40], b"\0x"].concat();
        let cases: [Case; 9] = [
            (b"blob 2\0hi", Ok((Kind::Blob, b"hi"))),
            (b"tree 0\0", Ok((Kind::Tree, b""))),
            (b"blob 02\0hi", Err("loose header")),
            (b"blob \0", Err("loose header")),
            (b"blob 2x\0hi", Err("loose header")),
            (b"blob2\0hi", Err("loose header")),
            (b"note 2\0hi", Err("loose header")),
            (b"no header here", Err("loose header")),
            (&long_size, Err("loose header")),
        ];
        for (raw, expected) in cases {
            assert_eq!(
                read("header", &deflated(raw), None),
                expected
                    .map(|(kind, data)| (kind, data.to_vec()))
                    .map_err(str::to_string),
                "{}",
                String::from_utf8_lossy(raw)
            );
        }
    }

    #[test]
    fn a_size_past_what_fits_is_refused_rather_than_wrapped() {
        let raw = format!("blob {}0\0", usize::MAX);
        assert_eq!(
            read("wrapped", &deflated(raw.as_bytes()), None),
            Err("loose header".to_string()),
            "a size past what fits was wrapped round to one that does"
        );
    }

    #[test]
    fn a_stream_is_inflated_no_further_than_its_header_says_nor_taken_short_of_it() {
        // Cut halfway, so a reader inflating the whole stream finds it ends
        // early; only one stopping at the size its header gives refuses it
        // for its size, long before the cut.
        let mut bomb = deflated(&[b"blob 3\0".as_slice(), &vec![0; 8 << 20]].concat());
        bomb.truncate(bomb.len() / 2);
        // Cut inside the checksum that closes it, after every byte it gives.
        let mut cut = deflated(b"blob 2\0hi");
        cut.truncate(cut.len() - 2);
        let trailed = [deflated(b"blob 2\0hi").as_slice(), b"x"].concat();
        let cases = [
            (trailed, "garbage at end of loose object"),
            (bomb, "loose object's size"),
            (deflated(b"blob 9\0hi"), "loose object's size"),
            (cut, "loose object's stream ends early"),
            (b"not zlib".to_vec(), "loose object's stream"),
        ];
        for (stream, refused) in cases {
            // 64 KiB is room for any header and the object it says, not for
            // what the bomb inflates to. Apart, so a read that spins fails
            // by its message.
            let heard =
                crate::regular_file::tests::apart(move || read("bomb", &stream, Some(64 << 10)));
            assert_eq!(
                heard.expect("a loose read was still inflating after 30 s"),
                Err(refused.to_string()),
                "a stream inflating past or short of its header's size, not zlib, or with \
                 bytes after it, wasn't refused at it"
            );
        }
    }
}
