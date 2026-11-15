//! Reftables written here as git lays them out, a table to a stack, so the
//! reading is proven without git; the reading's own tests build a large repository's
//! layout from them too.

use std::fs;
use std::path::Path;

use super::*;

/// A number as a reftable spells it, the inverse of `varint`.
fn spelled(mut value: usize) -> Vec<u8> {
    let mut bytes = vec![u8::try_from(value & 0x7f).expect("seven bits")];
    value >>= 7;
    while value > 0 {
        value -= 1;
        bytes.push(0x80 | u8::try_from(value & 0x7f).expect("seven bits"));
        value >>= 7;
    }
    bytes.reverse();
    bytes
}

fn unhex(commit: &str) -> Vec<u8> {
    (0..commit.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&commit[at..at + 2], 16).expect("hex"))
        .collect()
}

/// One table of a single ref block, its records in the order given (git
/// sorts them by name; the reading doesn't need it), each sharing what it
/// can of the name before it.
pub(crate) fn table(records: &[(&str, Option<Ref>)]) -> Vec<u8> {
    let mut header = b"REFT".to_vec();
    header.push(1);
    header.extend([0, 0x10, 0]);
    header.extend([0; 16]);
    let mut block = vec![b'r', 0, 0, 0];
    let mut before: &str = "";
    for (name, value) in records {
        let shared = before
            .bytes()
            .zip(name.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        let suffix = &name.as_bytes()[shared..];
        let kind = match value {
            None => 0,
            Some(Ref::Commit(_)) => 1,
            Some(Ref::Symbolic(_)) => 3,
        };
        block.extend(spelled(shared));
        block.extend(spelled(suffix.len() << 3 | kind));
        block.extend(suffix);
        block.extend(spelled(1));
        match value {
            None => {}
            Some(Ref::Commit(commit)) => block.extend(unhex(commit)),
            Some(Ref::Symbolic(target)) => {
                block.extend(spelled(target.len()));
                block.extend(target.as_bytes());
            }
        }
        before = name;
    }
    let first = u32::try_from(header.len() + 4).expect("small");
    block.extend(&first.to_be_bytes()[1..]);
    block.extend(1u16.to_be_bytes());
    let length = u32::try_from(header.len() + block.len()).expect("small");
    block[1..4].copy_from_slice(&length.to_be_bytes()[1..]);
    let mut file = header.clone();
    file.extend(block);
    // A log block follows, which the reading stops at.
    file.extend([b'g', 0, 0, 4]);
    file.extend(header);
    file.extend([0; 44]);
    file
}

/// A stack of tables in `folder`, oldest first.
pub(crate) fn write_stack(folder: &Path, tables: &[Vec<u8>]) {
    fs::create_dir_all(folder).expect("the reftable folder");
    let mut list = String::new();
    for (at, table) in tables.iter().enumerate() {
        let name = format!("0x{at:012x}-0x{at:012x}-00000000.ref");
        fs::write(folder.join(&name), table).expect("a table written");
        list.push_str(&name);
        list.push('\n');
    }
    fs::write(folder.join("tables.list"), list).expect("the list written");
}

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const OTHER: &str = "89abcdef0123456789abcdef0123456789abcdef";

#[test]
fn a_number_reads_as_it_was_spelled() {
    for value in [0, 1, 127, 128, 149, 16_511, 16_512, 2_000_000] {
        let bytes = spelled(value);
        let mut at = 0;
        assert_eq!(varint(&bytes, &mut at), Some(value), "{bytes:?}");
        assert_eq!(at, bytes.len(), "{value}");
    }
    let mut at = 0;
    assert_eq!(
        varint(&[0x80, 0x15], &mut at),
        Some(149),
        "git's own spelling of 149"
    );
}

#[test]
fn a_stack_reads_its_symbolic_refs_and_commits_the_newest_standing() {
    let folder = std::env::temp_dir().join(format!("repos-{}-a-stack", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    write_stack(
        &folder,
        &[
            table(&[
                ("HEAD", Some(Ref::Symbolic("refs/heads/main".to_owned()))),
                ("refs/heads/main", Some(Ref::Commit(COMMIT.to_owned()))),
                ("refs/heads/old", Some(Ref::Commit(COMMIT.to_owned()))),
            ]),
            table(&[
                ("refs/heads/main", Some(Ref::Commit(OTHER.to_owned()))),
                ("refs/heads/old", None),
            ]),
        ],
    );
    let refs = stack(&folder).expect("the stack reads");
    assert_eq!(
        refs.get("HEAD"),
        Some(&Ref::Symbolic("refs/heads/main".to_owned())),
        "{refs:#?}"
    );
    assert_eq!(
        refs.get("refs/heads/main"),
        Some(&Ref::Commit(OTHER.to_owned())),
        "{refs:#?}"
    );
    assert_eq!(
        refs.get("refs/heads/old"),
        None,
        "a deletion didn't stand: {refs:#?}"
    );
}

#[test]
fn a_file_that_isnt_a_reftable_is_refused() {
    let mut refs = HashMap::new();
    assert_eq!(records(b"not a table at all, just words", &mut refs), None);
    assert_eq!(records(&[], &mut refs), None);
}
