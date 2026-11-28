//! What Nix and the machine queue read: the one builder lent, if any, for
//! every build; and beside it the builders standing by that answer now, for
//! a build that asks for them by naming their file.
//!
//! Standing builders are never offered to every build, since they may be
//! machines whose owner wants only some work on them: a build of private
//! code must never reach one by default. Builders beside are the other
//! kind, machines that may take any of this one's work, and are named to
//! every build while they answer, for their big work, as the one lent is.
//! A builder beside that fails two proofs in a row (src/proof.rs) is left
//! out of every list until one passes.
//!
//! While the builder lent for every build fails its proofs, or none is
//! lent, the file for any work names stand-ins that passed theirs: the
//! second lent builder, else the warm VM builders, in their own lines for
//! any work. The one lent here is named alone again once it passes.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::cloud::{Cloud, Place};
use crate::freeing::{self, Freeing};
use crate::proof::{self, Health, Keeper, State};

mod lines;
mod named;
mod reaching;
mod resolving;
mod trusted;

pub use lines::{Standing, standing};
use lines::{for_any_work, for_big_work, text};
use named::{Named, for_anything, say_named, stalled_alike, stand_ins, tell, told};
pub use reaching::{forget_reached, nearby_answers};
use resolving::answers;
use trusted::{Trust, kept_beside};

#[cfg(not(test))]
const STANDING: &str = proof::STANDING;
#[cfg(test)]
const STANDING: &str = "/nonexistent/nix-offload-standing/health.json";

fn seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// A file of machines lines, written only when what it should say changes.
struct File {
    path: PathBuf,
    written: Option<String>,
}

impl File {
    fn say(&mut self, text: String) {
        if self.written.as_ref() != Some(&text) {
            write(&self.path, &text);
            self.written = Some(text);
        }
    }
}

/// The builder lent, as Nix is told of it: its machines line, and the
/// store a whole build may be sent to, if the helper gave one.
#[derive(Clone, Copy)]
pub struct Lending<'a> {
    pub line: &'a str,
    pub store: Option<&'a str>,
}

/// The two machines files: `every` for every build, holding only the one
/// lent, and `with_standing` for a build that names it, holding the
/// standing builders that answer as well; and `store`, the lent builder's
/// store, for a build sent there whole (`--store`), so only its
/// derivations cross a slow connection rather than what's built here; and
/// `lending`, the builder being made, for the machine queue to hold builds
/// for. Nix's build hook reads its builders once, when a build starts, and
/// an empty list keeps that whole build here, even once a builder stands.
pub struct Machines {
    every: File,
    with_standing: File,
    store: File,
    lending: File,
    standing: Vec<Standing>,
    beside: Vec<Standing>,
    /// The files `standing` and `beside` are read again from, once
    /// `read_from` gives them.
    lists: [Option<PathBuf>; 2],
    /// Where the builders beside that answer are named alone, once
    /// `beside` gives it.
    alone: Option<File>,
    /// Where the one lent is named for any work, once `for_anything`
    /// gives it.
    anything: Option<File>,
    /// The line of the one lent as `proven` last said it, and how its
    /// proofs stood then.
    proven: (Option<String>, State),
    /// Whether the one lent built its last proof but couldn't carry its
    /// copy up, as `way_stalled` last said.
    way_stalled: bool,
    /// The second lent builder's folder and the VM builders', whose
    /// builders stand in for the one lent here, once `stand_ins_from`
    /// gives them.
    stand_ins: Option<(PathBuf, Vec<PathBuf>)>,
    /// Who the file for any work named last.
    named: Named,
    /// The program told when no builder answers any work, and again once
    /// one does, once `telling` gives it.
    telling: Option<PathBuf>,
    /// Each builder beside's proofs, by its line.
    kept: Vec<(String, Keeper)>,
    /// How each builder beside stood when the lender before this one last
    /// said, once `going_on_from` gives it.
    earlier: Vec<Health>,
    /// The nix the builders beside are proven through.
    nix: String,
    /// Where root's prover says how the builders beside stand.
    root: PathBuf,
    /// Whether Nix trusts this account to prove the builders beside here.
    trust: Trust,
    /// The file naming what frees each builder beside, read again with
    /// `lists`, once `freeing_from` gives it.
    freeing_from: Option<PathBuf>,
    /// What frees each builder beside that names one, by its address.
    freeing: Vec<(String, Freeing)>,
    /// Where the one lent is said to be in the cloud, for the service
    /// that writes root's lines, once `in_cloud` gives the files.
    cloud: Option<Cloud>,
    /// Where the one lent is, as `placing` last said.
    place: Option<Place>,
}

impl Machines {
    pub fn new(
        every: PathBuf,
        with_standing: PathBuf,
        store: PathBuf,
        lending: PathBuf,
        standing: Vec<Standing>,
    ) -> Self {
        let file = |path| File {
            path,
            written: None,
        };
        Self {
            every: file(every),
            with_standing: file(with_standing),
            store: file(store),
            lending: file(lending),
            standing,
            beside: Vec::new(),
            lists: [None, None],
            alone: None,
            anything: None,
            proven: (None, State::None),
            way_stalled: false,
            stand_ins: None,
            named: Named::Nothing,
            telling: None,
            kept: Vec::new(),
            earlier: Vec::new(),
            nix: "nix".to_owned(),
            root: PathBuf::from(STANDING),
            trust: Trust::default(),
            freeing_from: None,
            freeing: Vec::new(),
            cloud: None,
            place: None,
        }
    }

    /// With `cloud`, where the one lent is said to be, written before
    /// any machines file names it and emptied once none is lent, as
    /// [`crate::cloud`] says.
    #[must_use]
    pub fn in_cloud(self, cloud: Cloud) -> Self {
        Self {
            cloud: Some(cloud),
            ..self
        }
    }

    /// Say where the one lent is, for the next [`Machines::show`].
    pub fn placing(&mut self, place: Option<Place>) {
        self.place = place;
    }

    /// With the file naming what frees each builder beside, read again
    /// each time it names who answers, as [`freeing::programs`] reads it:
    /// a builder beside whose proof finds it full is freed by its program.
    #[must_use]
    pub fn freeing_from(self, path: Option<PathBuf>) -> Self {
        Self {
            freeing_from: path,
            ..self
        }
    }

    /// With the folders whose builders stand in for the one lent here in
    /// the file for any work while it fails its proofs or none is lent: the
    /// `second` lent builder's, then the `vm` builders'.
    #[must_use]
    pub fn stand_ins_from(self, second: PathBuf, vm: Vec<PathBuf>) -> Self {
        Self {
            stand_ins: Some((second, vm)),
            ..self
        }
    }

    /// With `program` told, as [`told`] says, when no builder answers any
    /// work, and again once one does.
    #[must_use]
    pub fn telling(self, program: Option<PathBuf>) -> Self {
        Self {
            telling: program,
            ..self
        }
    }

    /// Going on from `earlier`, how each builder beside stood when the
    /// lender before this one last said, until its own proofs say.
    #[must_use]
    pub fn going_on_from(self, earlier: Vec<Health>) -> Self {
        Self { earlier, ..self }
    }

    /// With `nix` to prove the builders beside through.
    #[cfg(test)]
    #[must_use]
    pub fn proving_with(self, nix: &str) -> Self {
        Self {
            nix: nix.to_owned(),
            ..self
        }
    }

    /// With `root` where root's prover says how the builders beside stand.
    #[cfg(test)]
    #[must_use]
    pub fn proven_by_root_in(self, root: PathBuf) -> Self {
        Self { root, ..self }
    }

    /// Say how the proofs of the one lent, whose line is `line`, stand, or
    /// of the last, once it's gone.
    pub fn proven(&mut self, line: Option<&str>, state: State) {
        self.proven = (line.map(str::to_owned), state);
    }

    /// Say whether the one lent built its last proof but couldn't carry
    /// its copy up.
    pub fn way_stalled(&mut self, stalled: bool) {
        self.way_stalled = stalled;
    }

    /// How the builders beside stand, where there are builders beside.
    pub fn beside_health(&self) -> Option<Vec<Health>> {
        self.alone.as_ref()?;
        Some(
            self.kept
                .iter()
                .map(|(_, keeper)| keeper.health.clone())
                .collect(),
        )
    }

    /// With `anything`, the one lent named alone with nothing mandatory,
    /// for a build that may send any public piece away rather than build
    /// it here, such as one with no local jobs; empty while none is lent.
    /// Never a builder beside or standing: only some work may reach them.
    #[must_use]
    pub fn for_anything(self, anything: PathBuf) -> Self {
        Self {
            anything: Some(File {
                path: anything,
                written: None,
            }),
            ..self
        }
    }

    /// With `beside`, builders standing by whose owner lets them take any
    /// of this machine's work, so they're named to every build while they
    /// answer, and never to a build that asks for the standing ones; and
    /// named alone, as themselves, in `alone`, for a build that may go to
    /// them and nowhere else, empty while none answers.
    #[must_use]
    pub fn beside(self, alone: PathBuf, beside: Vec<Standing>) -> Self {
        Self {
            beside,
            alone: Some(File {
                path: alone,
                written: None,
            }),
            ..self
        }
    }

    /// With the files the standing builders and those beside are read
    /// from, read again each time it names who answers, so a switch that
    /// changes who they are is taken up at once. A lender superseded waits
    /// for its builder to go idle before it starts again, which under load
    /// on 4 November kept a switch's new builders unnamed 2 h 41 min. One
    /// that can't be read leaves its builders as they were.
    #[must_use]
    pub fn read_from(self, standing: Option<PathBuf>, beside: Option<PathBuf>) -> Self {
        Self {
            lists: [standing, beside],
            ..self
        }
    }

    fn reread(&mut self) {
        let [standing, beside] = self.lists.each_ref().map(|path| {
            fs::read_to_string(path.as_ref()?)
                .ok()
                .map(|text| self::standing(&text))
        });
        if let Some(standing) = standing {
            self.standing = standing;
        }
        if let Some(beside) = beside {
            self.beside = beside;
        }
        let Some(programs) = self
            .freeing_from
            .as_ref()
            .and_then(|path| fs::read_to_string(path).ok())
            .map(|text| freeing::programs(&text))
        else {
            return;
        };
        // A run under way is kept while its builder still names the same
        // program, so a reread never strands one.
        let mut was = std::mem::take(&mut self.freeing);
        for (address, program) in programs {
            let kept = was
                .iter()
                .position(|(was, freeing)| *was == address && freeing.program() == program);
            let freeing = kept.map_or_else(|| Freeing::of(&program), |at| was.swap_remove(at).1);
            self.freeing.push((address, freeing));
        }
    }

    /// Say the builder being made, a line of its own, or nothing once it
    /// stands or won't.
    pub fn asking(&mut self, name: Option<&str>) {
        self.lending
            .say(name.map_or_else(String::new, |name| format!("{name}\n")));
    }

    /// Say the one lent, and those beside that answer, to every build, each
    /// for its big work; the one lent with the standing builders that
    /// answer to a build that asks for them; and its store, a line of its
    /// own, or nothing.
    pub fn show(&mut self, lending: Option<Lending>) {
        self.reread();
        // First, so the service never meets a line before its instance.
        if let Some(cloud) = &mut self.cloud {
            cloud.say(lending.and(self.place.as_ref()));
        }
        self.store.say(
            lending
                .and_then(|lending| lending.store)
                .map_or_else(String::new, |store| format!("{store}\n")),
        );
        if let Some(anything) = &mut self.anything {
            let any = lending.map(|lending| for_any_work(lending.line));
            let (line, state) = &self.proven;
            let state = match lending {
                Some(lending) if line.as_deref() == Some(lending.line) => *state,
                Some(_) => State::Unproven,
                None if line.is_none() => *state,
                None => State::None,
            };
            let (second, vm) = self
                .stand_ins
                .as_ref()
                .map(|(second, vm)| stand_ins(second, vm, seconds()))
                .unwrap_or_default();
            let shared = self.way_stalled
                && self
                    .stand_ins
                    .as_ref()
                    .is_some_and(|(second, vm)| stalled_alike(second, vm, seconds()));
            let (said, named) = for_anything(any.as_deref(), (state, shared), &second, &vm);
            if self.stand_ins.is_some() {
                say_named(self.named, named);
            }
            if let Some((word, sentence)) = told(self.named, named)
                && let Some(program) = &self.telling
            {
                tell(program, word, &sentence);
            }
            self.named = named;
            anything.say(said);
        }
        let lent = lending.map(|lending| for_big_work(lending.line));
        let lent = lent.as_deref();
        let answering = self
            .standing
            .iter()
            .filter(|builder| answers(&builder.address))
            .map(|builder| builder.line.as_str());
        self.with_standing.say(text(answering, lent));
        // Proofs are kept only of those still listed.
        self.kept
            .retain(|(line, _)| self.beside.iter().any(|builder| &builder.line == line));
        let now = Instant::now();
        let mut beside: Vec<&str> = Vec::new();
        for builder in &self.beside {
            if answers(&builder.address)
                && !kept_beside(
                    (&mut self.kept, &self.earlier),
                    &builder.line,
                    (&self.nix, &self.root, &mut self.trust),
                    now,
                )
            {
                beside.push(builder.line.as_str());
            }
        }
        for (address, freeing) in &mut self.freeing {
            let kept = self
                .beside
                .iter()
                .find(|builder| builder.address == *address);
            let full = kept.is_some_and(|builder| {
                self.kept.iter().any(|(line, keeper)| {
                    *line == builder.line && keeper.health.state() == State::Full
                })
            });
            let name = kept.map_or_else(|| address.clone(), |builder| proof::named(&builder.line));
            freeing.tend(&name, full, now);
        }
        if let Some(alone) = &mut self.alone {
            alone.say(text(beside.iter().copied(), None));
        }
        let big: Vec<String> = beside.iter().map(|line| for_big_work(line)).collect();
        self.every.say(text(big.iter().map(String::as_str), lent));
    }
}

/// Write the file whole or not at all.
fn write(path: &Path, text: &str) {
    let writing = path.with_extension("writing");
    let written = fs::write(&writing, text).and_then(|()| fs::rename(&writing, path));
    if let Err(error) = written {
        eprintln!("nix-offload: writing {}: {error}", path.display());
    }
}

#[cfg(test)]
mod tests;
