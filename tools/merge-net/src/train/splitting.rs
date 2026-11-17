//! What the train knows of its riders from the candidates it ran: which
//! of them a red candidate lies among, and since when no builder has
//! taken a candidate, so a run that checked nothing neither loses that
//! search nor holds the riders aboard for ever.

/// How long, in seconds, candidates may go untaken before the last one's
/// riders fall back to running again alone: as long as the net's watch
/// sends a run again for (15 sends two minutes apart). A count would be
/// gone in two minutes when nix-offload refuses at once, and the upload
/// hold of 16 November kept a candidate untaken for 22 minutes.
pub(super) const UNTAKEN_FOR: u64 = 30 * 60;

/// What follows a red candidate.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum After {
    /// The next candidate takes fewer riders.
    Narrows,
    /// The one rider that rode it alone falls.
    Falls,
    /// Every rider falls, since the train is red for more than one of them.
    AllFall,
}

/// What the train knows of its riders from the candidates it ran, so it
/// finds the rider that breaks a candidate by halves. A front half that
/// lands leaves the break among the riders after it, so the next takes
/// half of those rather than all of them again: forgetting that took
/// seven runs to find the fourth or fifth of five riders. A second rider
/// falling alone with nothing landed between says the train is red for
/// more than its riders, as for a main that broke or a gate that failed
/// to run, so every rider falls back to running again alone rather than
/// the train halving through each of them: about twice their number in
/// runs, where falling costs one each. Two riders that each break it,
/// boarded one after the other, read the same, and the riders behind
/// them fall back too: telling the two apart takes a run of main alone,
/// and falling back costs only what each paid before the train.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Splitting {
    /// How many riders, the first to board, a red candidate is known to
    /// lie among; none while no red is being searched.
    suspects: usize,
    /// Whether a rider fell riding alone since the train last landed.
    fell_alone: bool,
    /// When the first of the candidates in a row no builder took ended;
    /// none since one was taken.
    untaken_since: Option<u64>,
}

impl Splitting {
    /// How many riders, the first to board, the next candidate takes.
    pub(super) fn take(&self) -> usize {
        match self.suspects {
            0 => usize::MAX,
            suspects => (suspects / 2).max(1),
        }
    }

    /// A candidate of `riding` riders failed.
    pub(super) fn red(&mut self, riding: usize) -> After {
        if riding > 1 {
            self.suspects = riding;
            return After::Narrows;
        }
        let twice = self.fell_alone;
        *self = Self {
            suspects: 0,
            fell_alone: !twice,
            untaken_since: None,
        };
        if twice { After::AllFall } else { After::Falls }
    }

    /// A candidate no builder took ended `now`: whether the train tries
    /// again with its riders aboard and its search as it was, rather than
    /// that candidate's riders falling. Once candidates have gone untaken
    /// for `UNTAKEN_FOR` the search starts afresh, as those riders fall.
    pub(super) fn untaken(&mut self, now: u64) -> bool {
        let since = *self.untaken_since.get_or_insert(now);
        if now.saturating_sub(since) < UNTAKEN_FOR {
            return true;
        }
        *self = Self::default();
        false
    }

    /// A builder took a candidate, whatever it found.
    pub(super) fn taken(&mut self) {
        self.untaken_since = None;
    }

    /// A candidate of `riding` riders landed.
    pub(super) fn landed(&mut self, riding: usize) {
        self.suspects = self.suspects.saturating_sub(riding);
        self.fell_alone = false;
    }
}
