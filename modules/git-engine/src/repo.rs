//! One repository as the engine keeps it between questions: where its
//! folders are, and what it has read of its config, refs, objects, commit
//! graph and index, each kept until the files it came from change.

use crate::config::{Config, Context};
use crate::fsmonitor::{Monitor, Since};
use crate::graph::{Graph, Known, ahead_behind};
use crate::hash::Oid;
use crate::index::Index;
use crate::odb::{Kind, Odb, parse_commit};
use crate::refs::Refs;
use crate::reftable::Value;
use crate::stamp::{Stamp, Stamps};
use crate::status::{self, Counts, Limits, Paths, Settings};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Where a repository's folders are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub workdir: PathBuf,
    /// This worktree's git folder.
    pub git_dir: PathBuf,
    /// The git folder every worktree shares.
    pub common: PathBuf,
}

/// The repository holding `dir`, found as git finds it: the nearest folder
/// with a `.git` folder, or a `.git` file naming one.
pub fn discover(dir: &Path) -> Option<Location> {
    let mut at = Some(dir);
    while let Some(dir) = at {
        let dot = dir.join(".git");
        if let Ok(meta) = std::fs::metadata(&dot) {
            let git_dir = if meta.is_dir() {
                Some(dot)
            } else {
                std::fs::read_to_string(&dot)
                    .ok()
                    .and_then(|text| text.strip_prefix("gitdir:").map(|p| dir.join(p.trim())))
            };
            if let Some(git_dir) = git_dir
                && git_dir.join("HEAD").exists()
            {
                let common = std::fs::read_to_string(git_dir.join("commondir"))
                    .map(|text| git_dir.join(text.trim()))
                    .unwrap_or_else(|_| git_dir.clone());
                let common = std::fs::canonicalize(&common).unwrap_or(common);
                return Some(Location {
                    workdir: dir.to_path_buf(),
                    git_dir,
                    common,
                });
            }
        }
        at = dir.parent();
    }
    None
}

/// What one question about a repository is answered with: gitstatusd's
/// fields, in its order.
#[derive(Default, Debug)]
pub struct Answer {
    pub workdir: String,
    pub commit: String,
    pub local_branch: String,
    pub remote_branch: String,
    pub remote_name: String,
    pub remote_url: String,
    pub action: String,
    pub index_size: usize,
    /// Each count is None when the index wasn't read for it.
    pub counts: Option<Counts>,
    pub ahead: usize,
    pub behind: usize,
    pub stashes: usize,
    pub tag: String,
    pub push_remote_name: String,
    pub push_remote_url: String,
    pub push_ahead: usize,
    pub push_behind: usize,
    pub encoding: String,
    pub summary: String,
}

pub struct Options {
    pub limits: Limits,
    /// Past this many index entries, the worktree isn't looked at.
    pub max_index: Option<usize>,
    pub respect_untracked_setting: bool,
}

pub struct Repo {
    pub location: Location,
    home: Option<PathBuf>,
    config: Config,
    refs: Refs,
    odb: Odb,
    graph: Graph,
    known: Known,
    /// The index as last read, with its stamp and the checksum it ends in.
    index: Option<(Option<Stamp>, Arc<Index>, Vec<u8>)>,
    cache: status::Cache,
    walks: HashMap<(Oid, Oid), (usize, usize)>,
    /// HEAD's commit as last read: its name, tree, encoding and summary.
    head_commit: Option<(Oid, Oid, String, String)>,
    /// Every tag by name with the commit it comes to, and what they were
    /// read from.
    tags: Option<(Stamps, Vec<(String, Oid)>)>,
    /// git's file system monitor for this worktree, when it's asked to run
    /// one.
    monitor: Option<Monitor>,
}

fn config_files(location: &Location, home: Option<&Path>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(global) = std::env::var("GIT_CONFIG_GLOBAL") {
        files.push(PathBuf::from(global));
    } else {
        let xdg = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home.map(|h| h.join(".config")));
        if let Some(xdg) = xdg {
            files.push(xdg.join("git/config"));
        }
        if let Some(home) = home {
            files.push(home.join(".gitconfig"));
        }
    }
    files.push(location.common.join("config"));
    files
}

fn load_config(location: &Location, home: Option<&Path>, branch: Option<&str>) -> Config {
    let context = Context {
        git_dir: &location.git_dir,
        branch,
        home,
    };
    let mut files = config_files(location, home);
    let first = Config::load(&files, &context);
    if first.get_bool("extensions.worktreeconfig") == Some(true) {
        files.push(location.git_dir.join("config.worktree"));
        return Config::load(&files, &context);
    }
    first
}

fn is_reftable(config: &Config) -> bool {
    config
        .get("extensions.refstorage")
        .is_some_and(|s| s.eq_ignore_ascii_case("reftable"))
}

/// HEAD, read from the worktree's own file unless refs live in reftable.
fn head(location: &Location, refs: &Refs, reftable: bool) -> Option<Value> {
    if reftable {
        return refs.read("HEAD");
    }
    let text = std::fs::read(location.git_dir.join("HEAD")).ok()?;
    let text = text.trim_ascii_end();
    if let Some(target) = text.strip_prefix(b"ref:") {
        return Some(Value::Symbolic(
            String::from_utf8_lossy(target.trim_ascii()).into_owned(),
        ));
    }
    Oid::from_hex(text).map(Value::Oid)
}

fn expand_home(path: &str, home: Option<&Path>) -> PathBuf {
    match (path.strip_prefix("~/"), home) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(path),
    }
}

/// Maps a ref through a remote's fetch refspecs, as git finds a branch's
/// upstream: `refs/heads/main` through `refs/heads/*:refs/remotes/origin/*`.
fn map_refspec(refspec: &str, name: &str) -> Option<String> {
    let refspec = refspec.strip_prefix('+').unwrap_or(refspec);
    let (src, dst) = refspec.split_once(':')?;
    match (src.split_once('*'), dst.split_once('*')) {
        (Some((src_head, src_tail)), Some((dst_head, dst_tail))) => {
            let middle = name.strip_prefix(src_head)?.strip_suffix(src_tail)?;
            Some(format!("{dst_head}{middle}{dst_tail}"))
        }
        (None, None) if src == name => Some(dst.to_string()),
        _ => None,
    }
}

impl Repo {
    pub fn open(location: Location) -> io::Result<Repo> {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let config = load_config(&location, home.as_deref(), None);
        let refs = Refs::open(&location.git_dir, &location.common, is_reftable(&config))?;
        let objects = location.common.join("objects");
        let odb = Odb::open(&objects);
        let graph = Graph::open(&objects);
        Ok(Repo {
            location,
            home,
            config,
            refs,
            odb,
            graph,
            known: Known::default(),
            index: None,
            cache: status::Cache::default(),
            walks: HashMap::new(),
            head_commit: None,
            tags: None,
            monitor: None,
        })
    }

    /// Reads again whatever changed on disk since the last question.
    fn refresh(&mut self) -> io::Result<()> {
        if !crate::stamp::still(&self.config.stamps) {
            self.config = load_config(&self.location, self.home.as_deref(), None);
            self.refs = Refs::open(
                &self.location.git_dir,
                &self.location.common,
                is_reftable(&self.config),
            )?;
        } else if !self.refs.current() {
            self.refs = Refs::open(
                &self.location.git_dir,
                &self.location.common,
                is_reftable(&self.config),
            )?;
        }
        let objects = self.location.common.join("objects");
        if !crate::stamp::still(&self.odb.stamps) {
            self.odb = Odb::open(&objects);
        }
        if !crate::stamp::still(&self.graph.stamps) {
            self.graph = Graph::open(&objects);
            self.known = Known::default();
        }
        Ok(())
    }

    fn read_index(&mut self) -> io::Result<Option<Arc<Index>>> {
        let path = self.location.git_dir.join("index");
        let stamp = Stamp::of(&path);
        // An index written this second may be rewritten in the same one.
        if let Some((seen, index, _)) = &self.index
            && *seen == stamp
            && !stamp.is_some_and(|s| s.is_recent())
        {
            return Ok(Some(index.clone()));
        }
        let Some(stamp_now) = stamp else {
            self.index = None;
            return Ok(None);
        };
        let data = std::fs::read(&path)?;
        let meta = std::fs::metadata(&path)?;
        let written = {
            use std::os::unix::fs::MetadataExt;
            (meta.mtime(), meta.mtime_nsec())
        };
        // The same bytes written at the same moment are the same index, and
        // what was worked out against it still stands.
        let checksum = data[data.len().saturating_sub(32)..].to_vec();
        if let Some((_, index, seen)) = &self.index
            && *seen == checksum
            && index.written == written
        {
            let index = index.clone();
            self.index = Some((stamp, index.clone(), checksum));
            return Ok(Some(index));
        }
        let index = Arc::new(Index::parse(&data, written)?);
        self.index = Some((Some(stamp_now), index.clone(), checksum));
        Ok(Some(index))
    }

    fn ahead_behind(&mut self, local: Oid, upstream: Oid) -> io::Result<(usize, usize)> {
        if let Some(&answer) = self.walks.get(&(local, upstream)) {
            return Ok(answer);
        }
        let answer = ahead_behind(
            &self.graph,
            &mut self.known,
            &mut self.odb,
            &local,
            &upstream,
        )?;
        if self.walks.len() > 256 {
            self.walks.clear();
        }
        self.walks.insert((local, upstream), answer);
        Ok(answer)
    }

    fn action(&self) -> String {
        let dir = &self.location.git_dir;
        let has = |name: &str| dir.join(name).exists();
        let action = if has("rebase-merge") {
            if has("rebase-merge/interactive") {
                "rebase-i"
            } else {
                "rebase-m"
            }
        } else if has("rebase-apply") {
            if has("rebase-apply/rebasing") {
                "rebase"
            } else if has("rebase-apply/applying") {
                "am"
            } else {
                "am/rebase"
            }
        } else if has("MERGE_HEAD") {
            "merge"
        } else if has("REVERT_HEAD") {
            if has("sequencer/todo") {
                "revert-seq"
            } else {
                "revert"
            }
        } else if has("CHERRY_PICK_HEAD") {
            if has("sequencer/todo") {
                "cherry-seq"
            } else {
                "cherry"
            }
        } else if has("BISECT_LOG") {
            "bisect"
        } else {
            ""
        };
        // A rebase or am says how far through it is, as `rebase-i 2/5`.
        let step = |done: &str, all: &str| {
            let read = |name: &str| {
                std::fs::read_to_string(dir.join(name))
                    .ok()?
                    .trim()
                    .parse::<u64>()
                    .ok()
            };
            Some(format!(" {}/{}", read(done)?, read(all)?))
        };
        let progress = if has("rebase-merge") {
            step("rebase-merge/msgnum", "rebase-merge/end")
        } else if has("rebase-apply") {
            step("rebase-apply/next", "rebase-apply/last")
        } else {
            None
        };
        format!("{action}{}", progress.unwrap_or_default())
    }

    /// A tag pointing at `commit`, the first by name.
    fn tag(&mut self, commit: Oid) -> String {
        if self
            .tags
            .as_ref()
            .is_none_or(|(stamps, _)| !crate::stamp::still(stamps))
        {
            self.tags = Some(self.read_tags());
        }
        self.tags
            .as_ref()
            .and_then(|(_, tags)| tags.iter().find(|(_, peeled)| *peeled == commit))
            .map(|(name, _)| name.clone())
            .unwrap_or_default()
    }

    /// Every tag, sorted by name, with the commit it comes to once an
    /// annotated tag is peeled, and the stamps of what they were read from.
    fn read_tags(&mut self) -> (Stamps, Vec<(String, Oid)>) {
        let mut stamps = Stamps::new();
        let mut peeled: HashMap<String, Oid> = HashMap::new();
        let mut names: Vec<(String, Oid)> = match &self.refs {
            Refs::Reftable { shared, .. } => shared
                .refs
                .iter()
                .filter_map(|(name, value)| match value {
                    Value::Oid(oid) => name
                        .strip_prefix("refs/tags/")
                        .map(|n| (n.to_string(), *oid)),
                    Value::Symbolic(_) => None,
                })
                .collect(),
            Refs::Files { .. } => Vec::new(),
        };
        if let Refs::Reftable { shared, .. } = &self.refs {
            stamps.clone_from(&shared.stamps);
        }
        if let Refs::Files { shared, .. } = &self.refs {
            let mut found = std::collections::BTreeMap::new();
            let packed = shared.join("packed-refs");
            stamps.push((packed.clone(), Stamp::of(&packed)));
            if let Ok(text) = std::fs::read_to_string(&packed) {
                let mut last: Option<String> = None;
                for line in text.lines() {
                    // A `^` line gives the commit the tag above it peels to.
                    if let Some(hex) = line.strip_prefix('^')
                        && let Some(oid) = Oid::from_hex(hex.as_bytes())
                        && let Some(tag) = last.take()
                    {
                        peeled.insert(tag, oid);
                    } else if let Some((hex, name)) = line.split_once(' ')
                        && let Some(tag) = name.strip_prefix("refs/tags/")
                        && let Some(oid) = Oid::from_hex(hex.as_bytes())
                    {
                        found.insert(tag.to_string(), oid);
                        last = Some(tag.to_string());
                    } else {
                        last = None;
                    }
                }
            }
            let mut stack = vec![shared.join("refs/tags")];
            while let Some(dir) = stack.pop() {
                // A tag is written beside the others and renamed into place,
                // so its folder's stamp moves with every tag made or moved.
                stamps.push((dir.clone(), Stamp::of(&dir)));
                let Ok(entries) = std::fs::read_dir(&dir) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    if entry.file_type().is_ok_and(|t| t.is_dir()) {
                        stack.push(path);
                    } else if let Ok(rel) = path.strip_prefix(shared.join("refs/tags"))
                        && let Ok(text) = std::fs::read(&path)
                        && let Some(oid) = Oid::from_hex(text.trim_ascii())
                    {
                        // A loose tag is newer than its packed line, and
                        // so is what it peels to.
                        let name = rel.to_string_lossy().into_owned();
                        peeled.remove(&name);
                        found.insert(name, oid);
                    }
                }
            }
            names = found.into_iter().collect();
        }
        names.sort();
        let mut tags = Vec::with_capacity(names.len());
        for (name, mut oid) in names {
            if let Some(target) = peeled.get(&name) {
                tags.push((name, *target));
                continue;
            }
            // An annotated tag points at a tag object; peel it.
            for _ in 0..8 {
                let Ok(Some(object)) = self.odb.read(&oid) else {
                    break;
                };
                if object.kind != Kind::Tag {
                    break;
                }
                let target = object
                    .data
                    .strip_prefix(b"object ")
                    .and_then(|rest| Oid::from_hex(rest.get(..40)?));
                let Some(target) = target else { break };
                oid = target;
            }
            tags.push((name, oid));
        }
        (stamps, tags)
    }

    pub fn answer(&mut self, options: &Options, diff: bool) -> io::Result<Answer> {
        self.refresh()?;
        let reftable = is_reftable(&self.config);
        let head = head(&self.location, &self.refs, reftable);
        let branch = match &head {
            Some(Value::Symbolic(target)) => target.strip_prefix("refs/heads/").map(str::to_string),
            _ => None,
        };
        if self.config.asks_branch && self.config.branch != branch {
            self.config = load_config(&self.location, self.home.as_deref(), branch.as_deref());
        }
        let commit = match &head {
            Some(Value::Oid(oid)) => Some(*oid),
            Some(Value::Symbolic(target)) => self.refs.resolve(target),
            None => None,
        };
        let mut answer = Answer {
            workdir: self.location.workdir.to_string_lossy().into_owned(),
            commit: commit.map(|c| c.hex()).unwrap_or_default(),
            local_branch: branch.clone().unwrap_or_default(),
            action: self.action(),
            stashes: self.refs.stashes(),
            ..Answer::default()
        };
        let mut head_tree = None;
        if let Some(commit) = commit {
            // A commit never changes, so it's read once while it's HEAD.
            if self
                .head_commit
                .as_ref()
                .is_none_or(|(seen, ..)| *seen != commit)
                && let Some(object) = self.odb.read(&commit)?
                && object.kind == Kind::Commit
            {
                let parsed = parse_commit(&object.data)?;
                let summary = parsed.summary.replace(['\x1e', '\x1f'], " ");
                self.head_commit = Some((commit, parsed.tree, parsed.encoding, summary));
            }
            if let Some((seen, tree, encoding, summary)) = &self.head_commit
                && *seen == commit
            {
                head_tree = Some(*tree);
                answer.encoding.clone_from(encoding);
                answer.summary.clone_from(summary);
            }
        }
        if let (Some(branch), Some(commit)) = (&branch, commit) {
            let remote = self
                .config
                .get(&format!("branch.{branch}.remote"))
                .map(str::to_string);
            let merge = self
                .config
                .get(&format!("branch.{branch}.merge"))
                .map(str::to_string);
            if let (Some(remote), Some(merge)) = (remote, merge) {
                let tracking = if remote == "." {
                    Some(merge.clone())
                } else {
                    let key = format!("remote.{remote}.fetch");
                    let mapped: Vec<String> = self
                        .config
                        .get_all(&key)
                        .filter_map(|spec| map_refspec(spec, &merge))
                        .collect();
                    mapped.into_iter().next()
                };
                if let Some(tracking) = tracking
                    && let Some(upstream) = self.refs.resolve(&tracking)
                {
                    answer.remote_name = remote.clone();
                    answer.remote_branch = merge
                        .strip_prefix("refs/heads/")
                        .unwrap_or(&merge)
                        .to_string();
                    answer.remote_url = self
                        .config
                        .get(&format!("remote.{remote}.url"))
                        .unwrap_or_default()
                        .to_string();
                    (answer.ahead, answer.behind) = self.ahead_behind(commit, upstream)?;
                }
            }
            let push_remote = self
                .config
                .get(&format!("branch.{branch}.pushremote"))
                .or_else(|| self.config.get("remote.pushdefault"))
                .map(str::to_string);
            if let Some(push_remote) = push_remote
                && let Some(pushed) = self
                    .refs
                    .resolve(&format!("refs/remotes/{push_remote}/{branch}"))
            {
                answer.push_remote_url = self
                    .config
                    .get(&format!("remote.{push_remote}.url"))
                    .unwrap_or_default()
                    .to_string();
                answer.push_remote_name = push_remote;
                (answer.push_ahead, answer.push_behind) = self.ahead_behind(commit, pushed)?;
            }
        }
        if let Some(commit) = commit {
            answer.tag = self.tag(commit);
        }
        if !diff {
            return Ok(answer);
        }
        // Before the first `git add` there's no index, and everything is
        // untracked.
        let index = self
            .read_index()?
            .unwrap_or_else(|| Arc::new(Index::empty()));
        answer.index_size = index.entries.len();
        let limits = options.limits;
        let mut counts = Counts::default();
        let staged_limits = Limits {
            staged: status::stop_at(limits.staged, status::conflicts(&index), limits.conflicted),
            ..limits
        };
        status::staged(&mut self.odb, &index, head_tree, staged_limits, &mut counts)?;
        if options
            .max_index
            .is_some_and(|max| index.entries.len() > max)
        {
            status::settle(&mut counts, limits);
            answer.counts = Some(counts);
            return Ok(answer);
        }
        let settings = Settings {
            autocrlf: self.config.get("core.autocrlf").map(str::to_string),
            filemode: self.config.get_bool("core.filemode").unwrap_or(true),
            trust_ctime: self.config.get_bool("core.trustctime").unwrap_or(true),
            show_untracked: !matches!(
                self.config
                    .get("status.showuntrackedfiles")
                    .map(str::to_ascii_lowercase)
                    .as_deref(),
                Some("no" | "false" | "off" | "0")
            ),
        };
        let xdg = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| self.home.as_ref().map(|h| h.join(".config")));
        let excludes_file = self
            .config
            .get("core.excludesfile")
            .map(|p| expand_home(p, self.home.as_deref()))
            .or_else(|| xdg.as_ref().map(|x| x.join("git/ignore")));
        let attributes_file = self
            .config
            .get("core.attributesfile")
            .map(|p| expand_home(p, self.home.as_deref()))
            .or_else(|| xdg.as_ref().map(|x| x.join("git/attributes")));
        // Asked before anything is looked at, so whatever changes after its
        // answer is named in the next one.
        if self.config.get_bool("core.fsmonitor") != Some(true) {
            self.monitor = None;
        } else if self.monitor.is_none() {
            self.monitor = Monitor::for_worktree(&self.location.git_dir);
        }
        let since = self.monitor.as_mut().map(Monitor::ask);
        if matches!(since, Some(Since::Anything)) {
            self.cache.lost_track();
        }
        let paths = Paths {
            workdir: &self.location.workdir,
            common: &self.location.common,
            excludes_file,
            attributes_file,
            monitored: self.monitor.is_some(),
            changed: match &since {
                Some(Since::Only(changed)) => Some(changed),
                _ => None,
            },
        };
        if settings.show_untracked || !options.respect_untracked_setting {
            status::untracked(&index, &paths, &mut self.cache, limits, &mut counts);
        }
        let unstaged_limits = Limits {
            unstaged: status::stop_at(limits.unstaged, counts.untracked, limits.untracked),
            ..limits
        };
        status::unstaged(
            &mut self.odb,
            &index,
            &paths,
            &settings,
            &mut self.cache,
            unstaged_limits,
            &mut counts,
        )?;
        status::settle(&mut counts, limits);
        answer.counts = Some(counts);
        Ok(answer)
    }
}
