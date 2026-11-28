//! One object store's packs, found through their own indexes or one
//! multi-pack-index, and the folder of its loose objects, which `loose`
//! reads. Where an object lies is found here; reading it is the odb's.

use super::invalid;
use crate::hash::Oid;
use crate::pack_index::{Index, Midx};
use crate::stamp::{Stamp, Stamps};
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub(super) struct Pack {
    pub(super) data: File,
    /// Its own index, for a pack the multi-pack-index doesn't cover.
    index: Option<Index>,
}

impl Pack {
    /// The pack beside the index `name` in `pack_dir`, or None where either
    /// won't open, the pack opened only once its index has and only where
    /// it's a file, so a FIFO in its place holds nothing up.
    fn indexed(pack_dir: &Path, name: &str) -> Option<Pack> {
        let index = Index::open(&pack_dir.join(name)).ok()?;
        let data = crate::regular_file::present(&pack_dir.join(name.replace(".idx", ".pack")));
        Some(Pack {
            data: data.ok()?,
            index: Some(index),
        })
    }
}

/// One object store: its packs and the folder of loose objects.
pub(super) struct Store {
    pub(super) dir: PathBuf,
    pub(super) midx: Option<Midx>,
    /// Packs by their `.idx` name; the multi-pack-index's come first, in its
    /// order.
    pub(super) packs: Vec<Pack>,
    /// The `.idx` name of each of `packs`, in their order.
    names: Vec<String>,
    pub(super) midx_packs: usize,
    /// The own indexes of the packs the multi-pack-index covers, in its
    /// order, read only once it can't answer a lookup.
    pub(super) covered: OnceLock<Vec<Option<Index>>>,
    /// Each pack's .idx and .pack, opened or not, stamped before it was,
    /// since a file rewritten in place, or freshened as git touches a pack
    /// rather than write again an object it holds, moves its own stamp but
    /// not its folder's. They're kept apart from the odb's own stamps, so
    /// one moving opens this store again (`Odb::forget_passing_marks`)
    /// rather than the odb, whose caches and repository stay.
    pub(super) stamps: Stamps,
}

/// The stamps of each index among `names` in `pack_dir`, and its pack's.
fn stamps_of<'a>(pack_dir: &Path, names: impl IntoIterator<Item = &'a String>) -> Stamps {
    let files = names.into_iter().flat_map(|name| {
        [
            pack_dir.join(name),
            pack_dir.join(name.replace(".idx", ".pack")),
        ]
    });
    files.map(|file| (file.clone(), Stamp::of(&file))).collect()
}

impl Store {
    #[expect(
        clippy::case_sensitive_file_extension_comparisons,
        reason = "git names a pack's index .idx, in lowercase"
    )]
    pub(super) fn open(dir: &Path) -> Store {
        let pack_dir = dir.join("pack");
        let midx = Midx::open(&pack_dir.join("multi-pack-index")).ok();
        let mut packs = Vec::new();
        let mut opened = Vec::new();
        let mut covered = std::collections::HashSet::new();
        let mut midx_packs = 0;
        let mut stamps = stamps_of(&pack_dir, midx.iter().flat_map(|midx| &midx.packs));
        if let Some(midx) = &midx {
            for name in &midx.packs {
                let data = pack_dir.join(name.replace(".idx", ".pack"));
                // A pack the index names but that's gone, or isn't a file,
                // ends the index's use.
                match crate::regular_file::present(&data) {
                    Ok(data) => packs.push(Pack { data, index: None }),
                    Err(_) => {
                        return Store::without_midx(dir);
                    }
                }
                opened.push(name.clone());
                covered.insert(name.clone());
            }
            midx_packs = packs.len();
        }
        let mut names: Vec<String> = std::fs::read_dir(&pack_dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|name| name.ends_with(".idx") && !covered.contains(name))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        stamps.extend(stamps_of(&pack_dir, &names));
        for name in names {
            // The pack is opened only once its index has, and only as a file.
            if let Some(pack) = Pack::indexed(&pack_dir, &name) {
                packs.push(pack);
                opened.push(name);
            }
        }
        Store {
            dir: dir.to_path_buf(),
            midx,
            packs,
            names: opened,
            midx_packs,
            covered: OnceLock::new(),
            stamps,
        }
    }

    #[expect(
        clippy::case_sensitive_file_extension_comparisons,
        reason = "git names a pack's index .idx, in lowercase"
    )]
    fn without_midx(dir: &Path) -> Store {
        let pack_dir = dir.join("pack");
        let mut names: Vec<String> = std::fs::read_dir(&pack_dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|name| name.ends_with(".idx"))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        let stamps = stamps_of(&pack_dir, &names);
        let (mut packs, mut opened) = (Vec::new(), Vec::new());
        for name in names {
            if let Some(pack) = Pack::indexed(&pack_dir, &name) {
                packs.push(pack);
                opened.push(name);
            }
        }
        Store {
            dir: dir.to_path_buf(),
            midx: None,
            packs,
            names: opened,
            midx_packs: 0,
            covered: OnceLock::new(),
            stamps,
        }
    }

    /// Whether it opened the packs `other` did, in the same order, so a
    /// place in one of them is the same place in the other.
    pub(super) fn opens_as(&self, other: &Store) -> bool {
        self.names == other.names && self.midx_packs == other.midx_packs
    }

    /// Where `oid` lies in this store's packs. An entry the
    /// multi-pack-index refuses, or one naming a pack past those it covers,
    /// can't say where the object is, so its packs' own indexes are read
    /// instead. What any index refuses is kept in `refused` rather than
    /// returned, the first kept standing, for the caller to let stand only
    /// once no other store, nor the loose objects, holds the object: a
    /// broken entry neither hides an object nor passes for a missing one.
    /// The packs in `bad` held a copy that wouldn't unpack, and are passed
    /// over as git passes over a copy it has marked bad.
    pub(super) fn locate(
        &self,
        oid: &Oid,
        bad: &[usize],
        refused: &mut Option<io::Error>,
    ) -> Option<(usize, u64)> {
        if let Some(midx) = &self.midx {
            let found = match midx.locate(oid) {
                // The multi-pack-index names one copy of each object, so
                // past a bad one only the packs it doesn't cover are read.
                Ok(Some((pack, offset))) if pack < self.midx_packs => {
                    (!bad.contains(&pack)).then_some((pack, offset))
                }
                Ok(None) => None,
                // The entry names a pack it doesn't cover, so the packs it
                // does are read; should none hold the object, that stands.
                Ok(Some(_)) => {
                    refused.get_or_insert_with(|| {
                        invalid("multi-pack-index names a pack past those it covers")
                    });
                    self.in_covered(oid, bad)
                }
                Err(error) => {
                    refused.get_or_insert(error);
                    self.in_covered(oid, bad)
                }
            };
            if found.is_some() {
                return found;
            }
        }
        for (at, pack) in self.packs.iter().enumerate().skip(self.midx_packs) {
            if bad.contains(&at) {
                continue;
            }
            match pack.index.as_ref().map(|index| index.offset(oid)) {
                Some(Ok(Some(offset))) => return Some((at, offset)),
                Some(Err(error)) => {
                    refused.get_or_insert(error);
                }
                Some(Ok(None)) | None => {}
            }
        }
        None
    }

    /// Where `oid` lies by the own indexes of the packs the
    /// multi-pack-index covers, read only once it can't answer and has
    /// left its own refusal standing. That refusal is what's said should
    /// none hold the object, so one that won't open or read is passed over
    /// rather than hiding the packs after it. A pack keeps its place
    /// whether or not its index opens, since the multi-pack-index numbers
    /// them. The packs in `bad` are passed over, as `locate` says.
    fn in_covered(&self, oid: &Oid, bad: &[usize]) -> Option<(usize, u64)> {
        let indexes = self.covered.get_or_init(|| {
            let pack_dir = self.dir.join("pack");
            self.midx
                .iter()
                .flat_map(|midx| &midx.packs)
                .map(|name| Index::open(&pack_dir.join(name)).ok())
                .collect()
        });
        indexes.iter().enumerate().find_map(|(at, index)| {
            if bad.contains(&at) {
                return None;
            }
            let offset = index.as_ref()?.offset(oid).ok()??;
            Some((at, offset))
        })
    }
}
