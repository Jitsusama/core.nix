# spend-watch

A Rust daemon that keeps the account told what its AI use costs. It reads
pi's session logs as they grow, the harness's subagent runs and its quests,
and the feeds a machine names for a provider's bill; keeps every dollar by
hour in `history.db`; and writes `status.json` and `collection.json` for the
shell, the overview's row, the margin and the collection on Super+U.
[`home.nix`][1] renders its config and runs it; the [contract][2] says what a
feed speaks.

## How It Decides

- **Figures.** Each hour's dollars come from the best source that has it: a
  feed's warehouse once it settles the hour, its live meter, pi's logs times
  each model's learned ratio of bill to list ("about"), and pi's logs at list
  prices where nothing has ever billed. Every source degrades on its own, so
  being cut off, signed out or offline leaves pi's logs, which always read.
  `src/series.rs` holds the order.
- **Tracing.** Every piece of spend carries its session, quest, repo, model,
  tool and agent kind, so every cut is a sum of the same pieces and every
  report agrees with every other. A quest's spend rolls up to its root.
- **Waste.** Each cache write gets a cause: a compaction, a pause longer
  than the cache lives, a fresh session, a model change. Rebuilt context is
  the money a session spent having again what it had.
- **Forecast.** A mixture of a state-space model on log daily spend and the
  same weekdays of the last eight weeks, weighted by each one's CRPS on the
  last 28 days. Picked by backtest over 175 days against the hour-of-week
  profile, which now only spreads a day across its hours. Its ranges are
  checked against what happened, not recalibrated, since spend changes faster
  than a calibration learns.
- **Caps.** A cap is a clock: simulated paths of the forecast give the
  chance of reaching it before the window rolls, and while over, when it
  comes back. Every change in allowance is history, and a cap lowered,
  lapsed, or a raise about to end is an alert.
- **Detectors.** Each threshold is set for a stated false-alarm rate. A
  burn is a minute CUSUM against twice the usual rate for the hour,
  alarming past the quantile eight weeks of daily maxima crossed once a
  month. Costly sessions are found against their peers with empirical-Bayes
  shrinkage and Benjamini-Hochberg at 5%. Changes of regime come from
  Bayesian online changepoints, confirmed after a week. `src/detect.rs`
  holds them all, and `spend-watch backtest` replays them on the history.
- **Sources.** A feed's source that can't be read is nothing for its first
  fifteen minutes, since a machine booting or waking fails every read until
  its network comes; then a note, or a warning when only a sign-in mends
  it. It clears on the first pass that finds it back. The history keeps
  since when each source has been as it is, so a restart doesn't reset it.
- **Strings.** Each finding carries a title for its row, a badge of a few
  words beside it, a note for the launcher's footer and a detail for what's
  copied and told. `src/collection.rs` lays them out as the launcher's
  other collections are: what wants a look first, then how things stand,
  each a figure with its parts a step in, then what was found.
- **Telling.** Notes stay in the collection. Warnings and alerts are told
  once as a notice, at most three a week outside a cap's, and withdrawn when
  they no longer hold or are marked expected. No notice holds an amount, and
  while the screen is shared the shell shows the quiet strings. Nothing is
  ever sent anywhere else.
- **Pace.** A pass costs about half a second on months of logs. It runs at
  most every 30 seconds while the logs grow and every five minutes while
  they don't; the heartbeat alone is written every 30 seconds, so the shell
  can tell a stopped daemon from a quiet one.

## Working on It

The crate is a library with a thin `main.rs`. Its tests run as it builds;
`direnv exec . cargo test` runs them here, and `spend-watch import` followed
by `show` or `report` tries it on the account's own logs without the daemon,
with a config whose state lies elsewhere.

[1]: home.nix
[2]: contract/README.md
