//! What the engine reads from a tree's and a commit's bytes, once the store
//! has given them whole.

use super::invalid;
use crate::hash::Oid;
use std::io;

/// A tree's entries, in its own order.
pub struct TreeEntry<'a> {
    pub mode: u32,
    pub name: &'a [u8],
    pub oid: Oid,
}

pub fn tree_entries(data: &[u8]) -> io::Result<Vec<TreeEntry<'_>>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let space = data[at..]
            .iter()
            .position(|&b| b == b' ')
            .ok_or_else(|| invalid("tree entry"))?;
        let mode = std::str::from_utf8(&data[at..at + space])
            .ok()
            .and_then(|m| u32::from_str_radix(m, 8).ok())
            .ok_or_else(|| invalid("tree mode"))?;
        at += space + 1;
        let nul = data[at..]
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| invalid("tree name"))?;
        let name = &data[at..at + nul];
        at += nul + 1;
        let oid = Oid::from_slice(data.get(at..at + 20).ok_or_else(|| invalid("tree oid"))?)
            .ok_or_else(|| invalid("tree oid"))?;
        at += 20;
        out.push(TreeEntry { mode, name, oid });
    }
    Ok(out)
}

/// What the engine reads from a commit.
pub struct Commit {
    pub tree: Oid,
    pub parents: Vec<Oid>,
    pub time: i64,
    pub encoding: String,
    pub summary: String,
}

pub fn parse_commit(data: &[u8]) -> io::Result<Commit> {
    let mut tree = None;
    let mut parents = Vec::new();
    let mut time = 0;
    let mut encoding = String::new();
    let mut lines = data.split(|&b| b == b'\n');
    let mut at = 0;
    for line in lines.by_ref() {
        at += line.len() + 1;
        if line.is_empty() {
            break;
        }
        if let Some(hex) = line.strip_prefix(b"tree ") {
            tree = Oid::from_hex(hex);
        } else if let Some(hex) = line.strip_prefix(b"parent ") {
            parents.push(Oid::from_hex(hex).ok_or_else(|| invalid("commit parent"))?);
        } else if let Some(who) = line.strip_prefix(b"committer ") {
            // "Name <mail> 1700000000 +0000"
            let close = who.iter().rposition(|&b| b == b'>').unwrap_or(0);
            time = std::str::from_utf8(&who[close + 1..])
                .ok()
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|t| t.parse().ok())
                .unwrap_or(0);
        } else if let Some(name) = line.strip_prefix(b"encoding ") {
            encoding = String::from_utf8_lossy(name).into_owned();
        }
    }
    let message = data.get(at.min(data.len())..).unwrap_or_default();
    // libgit2's summary: the first paragraph, its lines joined by spaces.
    let text = String::from_utf8_lossy(message);
    let summary = text
        .lines()
        .map(str::trim)
        .skip_while(|line| line.is_empty())
        .take_while(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    Ok(Commit {
        tree: tree.ok_or_else(|| invalid("commit without a tree"))?,
        parents,
        time,
        encoding,
        summary,
    })
}
