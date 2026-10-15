//! Commits' parents and generations: from the commit-graph where git has
//! written one, a single file or a chain of layers, and from the objects
//! themselves for commits newer than it. Counting how far a branch is ahead
//! of or behind its upstream walks both down in generation order, which
//! makes the counts exact rather than guessed from dates.

use crate::hash::Oid;
use crate::odb::{Kind, Odb, parse_commit};
use crate::stamp::{Stamp, Stamps};
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;
use std::path::Path;

const NO_PARENT: u32 = 0x7000_0000;

fn invalid(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("commit-graph: {what}"))
}

struct Layer {
    file: File,
    count: u32,
    fanout: u64,
    names: u64,
    data: u64,
    corrected: Option<u64>,
    overflow: Option<u64>,
    edges: Option<u64>,
}

impl Layer {
    fn open(path: &Path) -> io::Result<Layer> {
        let file = File::open(path)?;
        let mut head = [0; 8];
        file.read_exact_at(&mut head, 0)?;
        if head[..4] != *b"CGPH" || head[4] != 1 || head[5] != 1 {
            return Err(invalid("header"));
        }
        let chunks = usize::from(head[6]);
        let mut table = vec![0; 12 * (chunks + 1)];
        file.read_exact_at(&mut table, 8)?;
        let mut found = HashMap::new();
        for entry in table.as_chunks::<12>().0.iter().take(chunks) {
            let id: [u8; 4] = entry[..4].try_into().expect("4 bytes");
            found.insert(
                id,
                u64::from_be_bytes(entry[4..].try_into().expect("8 bytes")),
            );
        }
        let chunk = |id: &[u8; 4]| {
            found
                .get(id)
                .copied()
                .ok_or_else(|| invalid("missing chunk"))
        };
        let fanout = chunk(b"OIDF")?;
        let mut last = [0; 4];
        file.read_exact_at(&mut last, fanout + 255 * 4)?;
        Ok(Layer {
            count: u32::from_be_bytes(last),
            fanout,
            names: chunk(b"OIDL")?,
            data: chunk(b"CDAT")?,
            corrected: found.get(b"GDAT").copied(),
            overflow: found.get(b"GDO2").copied(),
            edges: found.get(b"EDGE").copied(),
            file,
        })
    }
}

pub struct Graph {
    layers: Vec<Layer>,
    /// Where each layer's positions begin.
    starts: Vec<u32>,
    /// Whether every layer has corrected commit dates, which then serve as
    /// generations; otherwise topological levels do.
    corrected: bool,
    pub stamps: Stamps,
}

impl Graph {
    pub fn open(objects: &Path) -> Graph {
        let single = objects.join("info/commit-graph");
        let chain = objects.join("info/commit-graphs/commit-graph-chain");
        let stamps = vec![
            (single.clone(), Stamp::of(&single)),
            (chain.clone(), Stamp::of(&chain)),
        ];
        let paths: Vec<_> = match std::fs::read_to_string(&chain) {
            Ok(text) => text
                .lines()
                .filter(|line| !line.is_empty())
                .map(|hash| objects.join(format!("info/commit-graphs/graph-{hash}.graph")))
                .collect(),
            Err(_) => vec![single],
        };
        let mut layers = Vec::new();
        for path in paths {
            match Layer::open(&path) {
                Ok(layer) => layers.push(layer),
                // A chain with a missing layer can't be trusted past it.
                Err(_) => break,
            }
        }
        let mut starts = Vec::new();
        let mut total = 0;
        for layer in &layers {
            starts.push(total);
            total += layer.count;
        }
        let corrected = !layers.is_empty() && layers.iter().all(|layer| layer.corrected.is_some());
        Graph {
            layers,
            starts,
            corrected,
            stamps,
        }
    }

    fn position(&self, oid: &Oid) -> io::Result<Option<u32>> {
        for (layer, start) in self.layers.iter().zip(&self.starts) {
            if let Some(at) = crate::odb::search(&layer.file, layer.fanout, layer.names, 20, oid)? {
                return Ok(Some(start + at));
            }
        }
        Ok(None)
    }

    fn layer_of(&self, position: u32) -> Option<(&Layer, u32)> {
        let at = self
            .starts
            .partition_point(|&start| start <= position)
            .checked_sub(1)?;
        let local = position - self.starts[at];
        (local < self.layers[at].count).then(|| (&self.layers[at], local))
    }

    fn commit(&self, position: u32) -> io::Result<(Vec<u32>, u64)> {
        let (layer, local) = self.layer_of(position).ok_or_else(|| invalid("position"))?;
        let mut data = [0; 36];
        layer
            .file
            .read_exact_at(&mut data, layer.data + 36 * u64::from(local))?;
        let word = |at: usize| u32::from_be_bytes(data[at..at + 4].try_into().expect("4 bytes"));
        let mut parents = Vec::new();
        let first = word(20);
        if first != NO_PARENT {
            parents.push(first);
        }
        let second = word(24);
        if second & 0x8000_0000 != 0 {
            let edges = layer.edges.ok_or_else(|| invalid("octopus without EDGE"))?;
            let mut at = u64::from(second & 0x7fff_ffff);
            loop {
                let mut raw = [0; 4];
                layer.file.read_exact_at(&mut raw, edges + 4 * at)?;
                let edge = u32::from_be_bytes(raw);
                parents.push(edge & 0x7fff_ffff);
                if edge & 0x8000_0000 != 0 {
                    break;
                }
                at += 1;
            }
        } else if second != NO_PARENT {
            parents.push(second);
        }
        let packed = u64::from_be_bytes(data[28..36].try_into().expect("8 bytes"));
        let time = packed & 0x3_ffff_ffff;
        let generation = if self.corrected {
            let corrected = layer.corrected.ok_or_else(|| invalid("GDAT"))?;
            let mut raw = [0; 4];
            layer
                .file
                .read_exact_at(&mut raw, corrected + 4 * u64::from(local))?;
            let offset = u32::from_be_bytes(raw);
            if offset & 0x8000_0000 == 0 {
                time + u64::from(offset)
            } else {
                let overflow = layer.overflow.ok_or_else(|| invalid("GDO2"))?;
                let mut raw = [0; 8];
                layer
                    .file
                    .read_exact_at(&mut raw, overflow + 8 * u64::from(offset & 0x7fff_ffff))?;
                time + u64::from_be_bytes(raw)
            }
        } else {
            packed >> 34
        };
        Ok((parents, generation))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Key {
    Graph(u32),
    Object(Oid),
}

pub struct Node {
    parents: Vec<Key>,
    generation: u64,
}

/// Finds commits, through the graph where it can and the objects where it
/// must, keeping each it has read.
struct Walker<'a> {
    graph: &'a Graph,
    odb: &'a mut Odb,
    nodes: &'a mut HashMap<Key, Node>,
}

impl Walker<'_> {
    fn key(&self, oid: &Oid) -> io::Result<Key> {
        Ok(self
            .graph
            .position(oid)?
            .map_or(Key::Object(*oid), Key::Graph))
    }

    fn node(&mut self, key: Key) -> io::Result<&Node> {
        if !self.nodes.contains_key(&key) {
            match key {
                Key::Graph(position) => {
                    let (parents, generation) = self.graph.commit(position)?;
                    let parents = parents.into_iter().map(Key::Graph).collect();
                    self.nodes.insert(
                        key,
                        Node {
                            parents,
                            generation,
                        },
                    );
                }
                Key::Object(oid) => self.read_objects(oid)?,
            }
        }
        Ok(&self.nodes[&key])
    }

    /// Commits the graph doesn't have, from `start` down to where the graph
    /// takes over, each with the generation git would write for it. With no
    /// graph at all a commit's date stands in, as libgit2 orders them.
    fn read_objects(&mut self, start: Oid) -> io::Result<()> {
        let dated = self.graph.layers.is_empty();
        let mut parsed: HashMap<Oid, (Vec<Key>, i64)> = HashMap::new();
        let mut stack = vec![start];
        while let Some(&oid) = stack.last() {
            if self.nodes.contains_key(&Key::Object(oid)) {
                stack.pop();
                continue;
            }
            if let std::collections::hash_map::Entry::Vacant(slot) = parsed.entry(oid) {
                let object = self
                    .odb
                    .read(&oid)?
                    .ok_or_else(|| invalid("commit missing"))?;
                if object.kind != Kind::Commit {
                    return Err(invalid("not a commit"));
                }
                let commit = parse_commit(&object.data)?;
                let parents = commit
                    .parents
                    .iter()
                    .map(|p| self.key(p))
                    .collect::<io::Result<Vec<_>>>()?;
                slot.insert((parents, commit.time));
            }
            let (parents, time) = parsed[&oid].clone();
            let time = u64::try_from(time).unwrap_or(0);
            if dated {
                self.nodes.insert(
                    Key::Object(oid),
                    Node {
                        parents,
                        generation: time,
                    },
                );
                stack.pop();
                continue;
            }
            let waiting: Vec<Oid> = parents
                .iter()
                .filter_map(|key| match key {
                    Key::Object(parent) if !self.nodes.contains_key(key) => Some(*parent),
                    _ => None,
                })
                .collect();
            if !waiting.is_empty() {
                stack.extend(waiting);
                continue;
            }
            let mut highest = 0;
            for &key in &parents {
                highest = highest.max(self.node(key)?.generation);
            }
            let generation = if self.graph.corrected {
                time.max(highest + 1)
            } else {
                highest + 1
            };
            self.nodes.insert(
                Key::Object(oid),
                Node {
                    parents,
                    generation,
                },
            );
            stack.pop();
        }
        Ok(())
    }
}

/// How many commits `local` has that `upstream` doesn't, and the reverse.
/// Commits already read, kept between walks for as long as the graph they
/// were found through stands.
#[derive(Default)]
pub struct Known(HashMap<Key, Node>);

pub fn ahead_behind(
    graph: &Graph,
    known: &mut Known,
    odb: &mut Odb,
    local: &Oid,
    upstream: &Oid,
) -> io::Result<(usize, usize)> {
    if local == upstream {
        return Ok((0, 0));
    }
    if known.0.len() > 500_000 {
        known.0.clear();
    }
    let mut walker = Walker {
        graph,
        odb,
        nodes: &mut known.0,
    };
    let mut flags: HashMap<Key, u8> = HashMap::new();
    let mut queue = BinaryHeap::new();
    let mut fresh = 0usize;
    for (oid, flag) in [(local, 1u8), (upstream, 2u8)] {
        let key = walker.key(oid)?;
        let generation = walker.node(key)?.generation;
        queue.push((generation, key));
        flags.insert(key, flag);
        fresh += 1;
    }
    let mut queued: std::collections::HashSet<Key> = queue.iter().map(|&(_, key)| key).collect();
    while fresh > 0 {
        let Some((_, key)) = queue.pop() else { break };
        queued.remove(&key);
        let flag = flags[&key];
        if flag != 3 {
            fresh -= 1;
        }
        let parents = walker.node(key)?.parents.clone();
        for parent in parents {
            let before = flags.get(&parent).copied().unwrap_or(0);
            let after = before | flag;
            if after == before {
                continue;
            }
            flags.insert(parent, after);
            if queued.contains(&parent) {
                if after == 3 {
                    fresh -= 1;
                }
            } else {
                let generation = walker.node(parent)?.generation;
                queue.push((generation, parent));
                queued.insert(parent);
                if after != 3 {
                    fresh += 1;
                }
            }
        }
    }
    let ahead = flags.values().filter(|&&flag| flag == 1).count();
    let behind = flags.values().filter(|&&flag| flag == 2).count();
    Ok((ahead, behind))
}
