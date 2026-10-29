//! A 64-bit EFI program as Authenticode sees it: what its signatures
//! cover, and the program with a signature added. Laid out as sbsign lays
//! it out, which every machine here already boots: the program padded to
//! eight bytes, then the certificate table padded to eight, then the PE
//! checksum recomputed over the whole. A table may hold several
//! signatures end to end, each padded to eight, as UEFI reads them: it
//! boots a program when any one is by a key its db holds, and the digest
//! each covers leaves the table out, so one is added without touching the
//! others.

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

/// A length or an offset as one of a PE file's 32-bit fields holds it: its
/// low 32 bits, all a PE file can say, so one past them wraps as it always
/// has.
fn dword(n: usize) -> u32 {
    let [first, second, third, fourth, ..] = n.to_le_bytes();
    u32::from_le_bytes([first, second, third, fourth])
}

fn padded(length: usize) -> usize {
    length.div_ceil(8) * 8
}

/// A program whose sections lie end to end after its headers, as ukify and
/// systemd lay theirs out, with no signature: those it had at its end, as
/// sbsign and this signer put them, are taken off and kept apart, so it
/// can be signed again, alone or beside them.
#[derive(Debug)]
pub struct Program {
    /// The program, padded with zeros to eight bytes.
    bytes: Vec<u8>,
    /// Where the checksum is, which the signature leaves out.
    checksum: usize,
    /// Where the certificate table's directory entry is, also left out.
    table: usize,
    /// The certificate table's entries it had, each whole and unpadded.
    signatures: Vec<Vec<u8>>,
}

/// The entries of a certificate table, each whole and unpadded, or why
/// they don't fill it.
fn entries(table: &[u8]) -> Result<Vec<Vec<u8>>, Fault> {
    let mut entries = vec![];
    let mut at = 0;
    while at < table.len() {
        let length = u32_at(table, at)? as usize;
        if length < 8 || at + length > table.len() {
            return Err(refused("its signatures don't fill their table"));
        }
        entries.push(table[at..at + length].to_vec());
        at += padded(length);
    }
    Ok(entries)
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
        let mut signatures = vec![];
        if (start, length) != (0, 0) {
            // Past the headers, which hold the table's own entry, and
            // ending where the program does.
            if start < headers.max(table + 8) || start.checked_add(length) != Some(bytes.len()) {
                return Err(refused("its signature isn't at its end"));
            }
            signatures = entries(&bytes[start..])?;
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
        placed.sort_unstable();
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
            signatures,
        })
    }

    /// The signatures it carried, each a certificate table entry whole.
    pub fn signatures(&self) -> &[Vec<u8>] {
        &self.signatures
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

    /// The program with this PKCS #7 signature its certificate table's
    /// only one.
    pub fn signed(self, signature: &[u8]) -> Vec<u8> {
        self.with_table(vec![entry(signature)])
    }

    /// The program with this PKCS #7 signature after those it carried that
    /// `keep` keeps.
    pub fn signed_beside(mut self, signature: &[u8], keep: impl Fn(&[u8]) -> bool) -> Vec<u8> {
        let mut table: Vec<Vec<u8>> = std::mem::take(&mut self.signatures)
            .into_iter()
            .filter(|kept| keep(kept))
            .collect();
        table.push(entry(signature));
        self.with_table(table)
    }

    /// The program with these entries, end to end, as its certificate
    /// table.
    fn with_table(mut self, table: Vec<Vec<u8>>) -> Vec<u8> {
        let mut certificates = vec![];
        for entry in table {
            let length = entry.len();
            certificates.extend(entry);
            certificates.resize(certificates.len() + padded(length) - length, 0);
        }

        let start = dword(self.bytes.len());
        self.bytes[self.table..self.table + 4].copy_from_slice(&start.to_le_bytes());
        self.bytes[self.table + 4..self.table + 8]
            .copy_from_slice(&dword(certificates.len()).to_le_bytes());
        self.bytes.extend(certificates);
        let sum = checksum(&self.bytes, self.checksum);
        self.bytes[self.checksum..self.checksum + 4].copy_from_slice(&sum.to_le_bytes());
        self.bytes
    }
}

/// A certificate table entry holding this PKCS #7 signature, unpadded.
fn entry(signature: &[u8]) -> Vec<u8> {
    let length = 8 + signature.len();
    let mut entry = Vec::with_capacity(length);
    entry.extend(dword(length).to_le_bytes());
    entry.extend(REVISION_2.to_le_bytes());
    entry.extend(PKCS_SIGNED_DATA.to_le_bytes());
    entry.extend(signature);
    entry
}

/// The certificate table a program carries at its end, as sbsign and this
/// signer put it, if it carries one.
pub fn signature(bytes: &[u8]) -> Option<&[u8]> {
    let pe = u32_at(bytes, 0x3c).ok()? as usize;
    let table = pe + 24 + 112 + 8 * CERTIFICATE_TABLE;
    let start = u32_at(bytes, table).ok()? as usize;
    let length = u32_at(bytes, table + 4).ok()? as usize;
    (start >= table + 8 && start.checked_add(length) == Some(bytes.len())).then(|| &bytes[start..])
}

/// The PE checksum: the file's 16-bit words summed with their carries
/// folded back in, its own field left out, plus the file's length.
fn checksum(bytes: &[u8], field: usize) -> u32 {
    // The carry goes back in at the bottom. A sum that carried is at most
    // 0xfffe, so adding the carry back never carries again.
    let fold = |sum: u16, word: u16| {
        let (wrapped, carried) = sum.overflowing_add(word);
        wrapped + u16::from(carried)
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
    u32::from(sum) + dword(bytes.len())
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
        let headers_u32 = u32::try_from(headers).unwrap();
        bytes[optional + 60..optional + 64].copy_from_slice(&headers_u32.to_le_bytes());
        bytes[optional + 108..optional + 112].copy_from_slice(&16u32.to_le_bytes());
        let header = optional + 240;
        bytes[header..header + 8].copy_from_slice(b".text\0\0\0");
        let section_u32 = u32::try_from(section).unwrap();
        bytes[header + 16..header + 20].copy_from_slice(&section_u32.to_le_bytes());
        bytes[header + 20..header + 24].copy_from_slice(&headers_u32.to_le_bytes());
        // Each byte its offset's lowest, never zero.
        for (n, byte) in bytes[headers..].iter_mut().enumerate() {
            *byte = n.to_le_bytes()[0] | 1;
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
    fn a_signature_beside_another_follows_it_each_in_whole_eights() {
        let bytes = program(0x400, 0);
        let once = Program::parse(bytes.clone()).unwrap().signed(&[0xab; 13]);
        let twice = Program::parse(once.clone())
            .unwrap()
            .signed_beside(&[0xcd; 5], |_| true);
        assert_eq!(twice.len(), bytes.len() + 24 + 16);
        assert_eq!(twice[bytes.len()..bytes.len() + 24], once[bytes.len()..]);
        assert_eq!(
            &twice[bytes.len() + 24..bytes.len() + 32],
            [13, 0, 0, 0, 0x00, 0x02, 0x02, 0x00]
        );
        let table = 0x98 + 112 + 32;
        assert_eq!(u32_at(&twice, table + 4).unwrap(), 40);
        assert_eq!(
            checksum(&twice, 0x98 + 64),
            u32_at(&twice, 0x98 + 64).unwrap()
        );
        let again = Program::parse(twice).unwrap();
        assert_eq!(again.covered(), Program::parse(bytes).unwrap().covered());
        assert_eq!(again.signatures().len(), 2);
        assert_eq!(again.signatures()[1][8..], [0xcd; 5]);
    }

    #[test]
    fn a_signature_beside_others_drops_those_not_kept_and_signed_alone_drops_all() {
        let once = Program::parse(program(0x400, 0))
            .unwrap()
            .signed(&[0xab; 13]);
        let replaced = Program::parse(once.clone())
            .unwrap()
            .signed_beside(&[0xcd; 5], |entry| entry[8] != 0xab);
        let parsed = Program::parse(replaced).unwrap();
        assert_eq!(parsed.signatures().len(), 1);
        assert_eq!(parsed.signatures()[0][8..], [0xcd; 5]);
        let twice = Program::parse(once)
            .unwrap()
            .signed_beside(&[0xcd; 5], |_| true);
        let alone = Program::parse(twice).unwrap().signed(&[0xef; 8]);
        assert_eq!(Program::parse(alone).unwrap().signatures().len(), 1);
    }

    #[test]
    fn a_table_its_entries_dont_fill_is_refused() {
        let mut signed = Program::parse(program(0x400, 0)).unwrap().signed(&[1; 8]);
        let end = signed.len();
        // The entry says it runs past the table's end.
        signed[end - 16..end - 12].copy_from_slice(&40u32.to_le_bytes());
        assert!(
            matches!(Program::parse(signed), Err(Fault::Unreadable { why, .. }) if why == "its signatures don't fill their table")
        );
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
    fn a_signature_said_to_start_inside_the_headers_is_refused() {
        let mut bytes = program(0x400, 0);
        let table = 0x98 + 112 + 32;
        bytes[table..table + 4].copy_from_slice(&4u32.to_le_bytes());
        let length = u32::try_from(bytes.len()).unwrap() - 4;
        bytes[table + 4..table + 8].copy_from_slice(&length.to_le_bytes());
        assert!(
            matches!(Program::parse(bytes.clone()), Err(Fault::Unreadable { why, .. }) if why == "its signature isn't at its end")
        );
        assert_eq!(signature(&bytes), None);
    }

    #[test]
    fn the_signature_at_a_programs_end_is_read() {
        let bytes = program(0x400, 0);
        assert_eq!(signature(&bytes), None);
        let signed = Program::parse(bytes.clone()).unwrap().signed(&[0xab; 13]);
        let table = signature(&signed).unwrap();
        assert_eq!(table.len(), 24);
        assert_eq!(table[8..21], [0xab; 13]);
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
