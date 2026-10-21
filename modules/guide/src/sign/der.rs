//! Just enough DER to take a certificate's issuer, serial and public key
//! out as the bytes they are, and to write a signature the way OpenSSL
//! writes one. Reading keeps every value's own encoding, so what goes into
//! the signature is the certificate's bytes, never a re-encoding of them.

use crate::Fault;

pub const INTEGER: u8 = 0x02;
pub const BIT_STRING: u8 = 0x03;
pub const OCTET_STRING: u8 = 0x04;
pub const NULL: u8 = 0x05;
pub const UTC_TIME: u8 = 0x17;
pub const GENERALIZED_TIME: u8 = 0x18;
pub const SEQUENCE: u8 = 0x30;
pub const SET: u8 = 0x31;

/// A value's tag and contents, and the whole of its encoding.
#[derive(Clone, Copy, Debug)]
pub struct Value<'a> {
    pub tag: u8,
    pub contents: &'a [u8],
    pub encoding: &'a [u8],
}

fn malformed(what: &str) -> Fault {
    Fault::unreadable(what, "it isn't DER")
}

/// The value at the start of `input`, and what follows it.
pub fn read<'a>(input: &'a [u8], what: &str) -> Result<(Value<'a>, &'a [u8]), Fault> {
    let (&tag, rest) = input.split_first().ok_or_else(|| malformed(what))?;
    // Tags past 30 take more bytes, and nothing read here has one.
    if tag & 0x1f == 0x1f {
        return Err(malformed(what));
    }
    let (&first, rest) = rest.split_first().ok_or_else(|| malformed(what))?;
    let (length, rest) = match first {
        // An indefinite length, which DER forbids.
        0x80 => return Err(malformed(what)),
        0..0x80 => (first as usize, rest),
        0x81..=0x84 => {
            let count = (first & 0x7f) as usize;
            if rest.len() < count || rest[0] == 0 {
                return Err(malformed(what));
            }
            let length = rest[..count]
                .iter()
                .fold(0usize, |n, byte| n << 8 | *byte as usize);
            if length < 0x80 {
                return Err(malformed(what));
            }
            (length, &rest[count..])
        }
        _ => return Err(malformed(what)),
    };
    if rest.len() < length {
        return Err(malformed(what));
    }
    let header = input.len() - rest.len();
    Ok((
        Value {
            tag,
            contents: &rest[..length],
            encoding: &input[..header + length],
        },
        &rest[length..],
    ))
}

/// Every value inside a constructed one, in order.
pub fn children<'a>(value: Value<'a>, what: &str) -> Result<Vec<Value<'a>>, Fault> {
    let mut rest = value.contents;
    let mut found = vec![];
    while !rest.is_empty() {
        let (child, after) = read(rest, what)?;
        found.push(child);
        rest = after;
    }
    Ok(found)
}

fn length(count: usize) -> Vec<u8> {
    if count < 0x80 {
        return vec![count as u8];
    }
    let bytes: Vec<u8> = count
        .to_be_bytes()
        .into_iter()
        .skip_while(|byte| *byte == 0)
        .collect();
    let mut encoded = vec![0x80 | bytes.len() as u8];
    encoded.extend(bytes);
    encoded
}

/// A value of this tag holding these parts, one after another.
pub fn value(tag: u8, parts: &[&[u8]]) -> Vec<u8> {
    let contents = parts.concat();
    let mut encoded = vec![tag];
    encoded.extend(length(contents.len()));
    encoded.extend(contents);
    encoded
}

pub fn sequence(parts: &[&[u8]]) -> Vec<u8> {
    value(SEQUENCE, parts)
}

/// A SET OF, its members in the order DER puts them: by their encodings.
pub fn set_of(mut members: Vec<Vec<u8>>) -> Vec<u8> {
    members.sort();
    let parts: Vec<&[u8]> = members.iter().map(Vec::as_slice).collect();
    value(SET, &parts)
}

/// An object identifier, from its arcs.
pub fn oid(arcs: &[u64]) -> Vec<u8> {
    let mut contents = vec![];
    let first = arcs[0] * 40 + arcs[1];
    for arc in std::iter::once(first).chain(arcs[2..].iter().copied()) {
        let mut groups = vec![(arc & 0x7f) as u8];
        let mut rest = arc >> 7;
        while rest > 0 {
            groups.push(0x80 | (rest & 0x7f) as u8);
            rest >>= 7;
        }
        contents.extend(groups.into_iter().rev());
    }
    value(0x06, &[&contents])
}

/// An algorithm as OpenSSL names one: its identifier and a NULL.
pub fn algorithm(arcs: &[u64]) -> Vec<u8> {
    sequence(&[&oid(arcs), &[NULL, 0]])
}

pub fn small_integer(n: u8) -> Vec<u8> {
    if n < 0x80 {
        vec![INTEGER, 1, n]
    } else {
        vec![INTEGER, 2, 0, n]
    }
}

/// A time as a signature's signing time holds it: UTCTime from 1950 to
/// 2049, GeneralizedTime otherwise, as OpenSSL chooses.
pub fn time(seconds: u64) -> Vec<u8> {
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    // Howard Hinnant's days-to-civil, from the days since 1970.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    let clock = format!(
        "{month:02}{day:02}{:02}{:02}{:02}Z",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    );
    if (1950..2050).contains(&year) {
        value(UTC_TIME, &[format!("{:02}{clock}", year % 100).as_bytes()])
    } else {
        value(GENERALIZED_TIME, &[format!("{year:04}{clock}").as_bytes()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_identifiers_are_encoded_as_der_has_them() {
        assert_eq!(
            oid(&[2, 16, 840, 1, 101, 3, 4, 2, 1]),
            [
                0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01
            ]
        );
        assert_eq!(
            oid(&[1, 3, 6, 1, 4, 1, 311, 2, 1, 4]),
            [
                0x06, 0x0a, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0x37, 0x02, 0x01, 0x04
            ]
        );
    }

    #[test]
    fn long_lengths_take_as_few_bytes_as_they_need() {
        assert_eq!(
            value(OCTET_STRING, &[&[0; 0x7f]])[..2],
            [OCTET_STRING, 0x7f]
        );
        assert_eq!(
            value(OCTET_STRING, &[&[0; 0x80]])[..3],
            [OCTET_STRING, 0x81, 0x80]
        );
        assert_eq!(
            value(OCTET_STRING, &[&[0; 0x1234]])[..4],
            [OCTET_STRING, 0x82, 0x12, 0x34]
        );
    }

    #[test]
    fn what_it_writes_it_reads_back_as_the_same_bytes() {
        let inner = value(OCTET_STRING, &[&[7; 300]]);
        let outer = sequence(&[&small_integer(1), &inner]);
        let (read_back, rest) = read(&outer, "a test").unwrap();
        assert!(rest.is_empty());
        assert_eq!(read_back.encoding, outer);
        let parts = children(read_back, "a test").unwrap();
        assert_eq!(parts[1].encoding, inner);
        assert_eq!(parts[1].contents, [7; 300]);
    }

    #[test]
    fn lengths_der_forbids_are_refused() {
        // Indefinite, padded with a zero, and long where short would do.
        for bad in [
            &[0x30, 0x80, 0, 0][..],
            &[0x04, 0x82, 0x00, 0x81],
            &[0x04, 0x81, 0x05, 1, 2, 3, 4, 5],
        ] {
            assert!(read(bad, "a test").is_err(), "{bad:02x?}");
        }
        assert!(read(&[0x04, 0x05, 1, 2], "a test").is_err());
    }

    #[test]
    fn a_set_of_is_sorted_by_encoding() {
        let set = set_of(vec![vec![0x30, 0x01, 0x02], vec![0x30, 0x01, 0x01]]);
        assert_eq!(set, [SET, 6, 0x30, 0x01, 0x01, 0x30, 0x01, 0x02]);
    }

    #[test]
    fn times_are_utc_until_2050_and_generalized_after() {
        // 2026-10-21 02:34:33, as sbsign wrote it.
        assert_eq!(time(1_792_550_073), value(UTC_TIME, &[b"261021023433Z"]));
        assert_eq!(time(951_782_400), value(UTC_TIME, &[b"000229000000Z"]));
        // 2050-01-01 00:00:00.
        assert_eq!(
            time(2_524_608_000),
            value(GENERALIZED_TIME, &[b"20500101000000Z"])
        );
    }
}
