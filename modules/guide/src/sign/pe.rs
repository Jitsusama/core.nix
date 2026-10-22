//! A 64-bit EFI program as Authenticode sees it: what its signature
//! covers, and the program with a signature added. Laid out as sbsign lays
//! it out, which every machine here already boots: the program padded to
//! eight bytes, then the certificate table padded to eight, then the PE
//! checksum recomputed over the whole.

use crate::Fault;

/// The certificate table's entry in the data directory.
const CERTIFICATE_TABLE: usize = 4;
const PE32_PLUS: u16 = 0x20b;
const SECTION_HEADER: usize = 40;
const REVISION_2: u16 = 0x0200;
const PKCS_SIGNED_DATA: u16 = 0x0002;

fn refused(why: &str) -> Fault {
    Fault::unreadable("the stick's boot program", why)
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, Fault> {
    bytes
        .get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| refused("it ends inside its own headers"))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, Fault> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| refused("it ends inside its own headers"))
}

fn padded(length: usize) -> usize {
    length.div_ceil(8) * 8
}

/// A program whose sections lie end to end after its headers, as ukify and
/// systemd lay theirs out, with no signature: one it had at its end, as
/// sbsign and this signer put it, is taken off, so it can be signed again.
#[derive(Debug)]
pub struct Program {
    /// The program, padded with zeros to eight bytes.
    bytes: Vec<u8>,
    /// Where the checksum is, which the signature leaves out.
    checksum: usize,
    /// Where the certificate table's directory entry is, also left out.
    table: usize,
}

impl Program {
    pub fn parse(mut bytes: Vec<u8>) -> Result<Program, Fault> {
        if bytes.get(..2) != Some(b"MZ") {
            return Err(refused("it isn't an EFI program"));
        }
        let pe = u32_at(&bytes, 0x3c)? as usize;
        if bytes.get(pe..pe + 4) != Some(b"PE\0\0") {
            return Err(refused("it isn't an EFI program"));
        }
        let sections = u16_at(&bytes, pe + 6)? as usize;
        let optional_size = u16_at(&bytes, pe + 20)? as usize;
        let optional = pe + 24;
        if u16_at(&bytes, optional)? != PE32_PLUS {
            return Err(refused("it isn't a 64-bit EFI program"));
        }
        let headers = u32_at(&bytes, optional + 60)? as usize;
        let directories = u32_at(&bytes, optional + 108)? as usize;
        let table = optional + 112 + 8 * CERTIFICATE_TABLE;
        if directories <= CERTIFICATE_TABLE || optional_size < 112 + 8 * (CERTIFICATE_TABLE + 1) {
            return Err(refused("it has nowhere to put a signature"));
        }
        let (start, length) = (
            u32_at(&bytes, table)? as usize,
            u32_at(&bytes, table + 4)? as usize,
        );
        if (start, length) != (0, 0) {
            if start == 0 || start.checked_add(length) != Some(bytes.len()) {
                return Err(refused("its signature isn't at its end"));
            }
            bytes.truncate(start);
            bytes[table..table + 8].fill(0);
        }
        let first_section = optional + optional_size;
        if headers > bytes.len() || first_section + sections * SECTION_HEADER > headers {
            return Err(refused("its headers run past where it says they end"));
        }
        let mut placed: Vec<(usize, usize)> = vec![];
        for section in 0..sections {
            let at = first_section + section * SECTION_HEADER;
            let size = u32_at(&bytes, at + 16)? as usize;
            let start = u32_at(&bytes, at + 20)? as usize;
            if size > 0 {
                placed.push((start, size));
            }
        }
        placed.sort();
        let mut end = headers;
        for (start, size) in placed {
            if start != end {
                return Err(refused("its sections don't lie end to end"));
            }
            end = start + size;
        }
        if end > bytes.len() {
            return Err(refused("a section runs past its end"));
        }
        bytes.resize(padded(bytes.len()), 0);
        Ok(Program {
            bytes,
            checksum: optional + 64,
            table,
        })
    }

    /// What the signature covers: everything but the checksum and the
    /// certificate table's entry, in the order the bytes lie.
    pub fn covered(&self) -> [&[u8]; 3] {
        [
            &self.bytes[..self.checksum],
            &self.bytes[self.checksum + 4..self.table],
            &self.bytes[self.table + 8..],
        ]
    }

    /// The program with this PKCS #7 signature in its certificate table.
    pub fn signed(mut self, signature: &[u8]) -> Vec<u8> {
        let length = 8 + signature.len();
        let mut certificates = Vec::with_capacity(padded(length));
        certificates.extend((length as u32).to_le_bytes());
        certificates.extend(REVISION_2.to_le_bytes());
        certificates.extend(PKCS_SIGNED_DATA.to_le_bytes());
        certificates.extend(signature);
        certificates.resize(padded(length), 0);

        let start = self.bytes.len() as u32;
        self.bytes[self.table..self.table + 4].copy_from_slice(&start.to_le_bytes());
        self.bytes[self.table + 4..self.table + 8]
            .copy_from_slice(&(certificates.len() as u32).to_le_bytes());
        self.bytes.extend(certificates);
        let sum = checksum(&self.bytes, self.checksum);
        self.bytes[self.checksum..self.checksum + 4].copy_from_slice(&sum.to_le_bytes());
        self.bytes
    }
}

/// The PE checksum: the file's 16-bit words summed with their carries
/// folded back in, its own field left out, plus the file's length.
fn checksum(bytes: &[u8], field: usize) -> u32 {
    let fold = |sum: u16, word: u16| {
        let wide = u32::from(sum) + u32::from(word);
        ((wide >> 16) + (wide & 0xffff)) as u16
    };
    let sum = [&bytes[..field], &bytes[field + 4..]]
        .iter()
        .flat_map(|part| part.chunks(2))
        .fold(0u16, |sum, word| {
            fold(
                sum,
                u16::from_le_bytes([word[0], *word.get(1).unwrap_or(&0)]),
            )
        });
    u32::from(sum) + bytes.len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smallest program the parser takes: headers, one section of
    /// `section` bytes, and the given trailing bytes after it.
    fn program(section: usize, after: usize) -> Vec<u8> {
        let headers = 0x200;
        let mut bytes = vec![0u8; headers + section + after];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&240u16.to_le_bytes());
        let optional = 0x98;
        bytes[optional..optional + 2].copy_from_slice(&PE32_PLUS.to_le_bytes());
        bytes[optional + 60..optional + 64].copy_from_slice(&(headers as u32).to_le_bytes());
        bytes[optional + 108..optional + 112].copy_from_slice(&16u32.to_le_bytes());
        let header = optional + 240;
        bytes[header..header + 8].copy_from_slice(b".text\0\0\0");
        bytes[header + 16..header + 20].copy_from_slice(&(section as u32).to_le_bytes());
        bytes[header + 20..header + 24].copy_from_slice(&(headers as u32).to_le_bytes());
        for (n, byte) in bytes[headers..].iter_mut().enumerate() {
            *byte = n as u8 | 1;
        }
        bytes
    }

    #[test]
    fn the_signature_covers_all_but_the_checksum_and_the_tables_entry() {
        let bytes = program(0x400, 0);
        let parsed = Program::parse(bytes.clone()).unwrap();
        let covered: usize = parsed.covered().iter().map(|part| part.len()).sum();
        assert_eq!(covered, bytes.len() - 12);
    }

    #[test]
    fn a_program_not_eight_byte_aligned_is_padded_and_the_padding_covered() {
        let bytes = program(0x400, 3);
        let parsed = Program::parse(bytes.clone()).unwrap();
        assert_eq!(parsed.bytes[..bytes.len()], bytes[..]);
        assert_eq!(parsed.bytes[bytes.len()..], [0; 5]);
        let covered: usize = parsed.covered().iter().map(|part| part.len()).sum();
        assert_eq!(covered, bytes.len() + 5 - 12);
    }

    #[test]
    fn the_signature_goes_after_the_program_in_whole_eights() {
        let bytes = program(0x400, 0);
        let signed = Program::parse(bytes.clone()).unwrap().signed(&[0xab; 13]);
        assert_eq!(signed.len(), bytes.len() + 24);
        assert_eq!(
            &signed[bytes.len()..bytes.len() + 8],
            [21, 0, 0, 0, 0x00, 0x02, 0x02, 0x00]
        );
        let table = 0x98 + 112 + 32;
        assert_eq!(u32_at(&signed, table).unwrap() as usize, bytes.len());
        assert_eq!(u32_at(&signed, table + 4).unwrap(), 24);
        assert_eq!(
            checksum(&signed, 0x98 + 64),
            u32_at(&signed, 0x98 + 64).unwrap()
        );
    }

    #[test]
    fn a_program_signed_already_is_read_as_it_was_before() {
        let bytes = program(0x400, 0);
        let signed = Program::parse(bytes.clone()).unwrap().signed(&[1; 8]);
        let again = Program::parse(signed).unwrap();
        assert_eq!(again.covered(), Program::parse(bytes).unwrap().covered());
    }

    #[test]
    fn a_signature_anywhere_but_the_end_is_refused() {
        let mut signed = Program::parse(program(0x400, 0)).unwrap().signed(&[1; 8]);
        signed.extend([0; 8]);
        assert!(
            matches!(Program::parse(signed), Err(Fault::Unreadable { why, .. }) if why == "its signature isn't at its end")
        );
    }

    #[test]
    fn sections_that_dont_lie_end_to_end_are_refused() {
        let mut bytes = program(0x400, 0x10);
        let start = 0x98 + 240 + 20;
        bytes[start..start + 4].copy_from_slice(&0x210u32.to_le_bytes());
        assert!(
            matches!(Program::parse(bytes), Err(Fault::Unreadable { why, .. }) if why.contains("end to end"))
        );
    }

    #[test]
    fn what_isnt_a_64_bit_program_is_refused() {
        assert!(Program::parse(b"#!/bin/sh\n".to_vec()).is_err());
        let mut bytes = program(0x400, 0);
        bytes[0x98..0x9a].copy_from_slice(&0x10bu16.to_le_bytes());
        assert!(Program::parse(bytes).is_err());
    }
}
