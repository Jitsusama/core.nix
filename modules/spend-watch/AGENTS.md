# spend-watch

A Rust daemon that keeps the account told what its AI use costs. It reads
pi's session logs as they grow, its agents' as well (a swarm's, a review
council's), the harness's subagent runs and its quests, and the feeds a
machine names for a provider's bill; keeps every dollar by
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
  report agrees with every other. A quest's spend rolls up to its root. A
  bill that doesn't say which session spent it is shared by the pieces on
  its model, by their list prices, since models are priced apart; a
  metered hour keeps what other tools spent on each model as theirs on it.
  When pi carries a session into a new log it copies the old entries ahead
  of the new header, so what comes before the header is the parent log's
  to count, unless that log is gone.
- **Agents.** The config names where agents' logs are kept, by patterns
  whose each `*` is one name, so a large tree beside them is never walked,
  and the kind of work each pattern's are. A run's agents (a council's
  reviewers, a judge) are that run's work, under the session that started
  it and in its quest, at the hours their logs have turns in, and the
  run's own total counts only what no log of it covers, as a fleet's,
  which keeps none. Any other agent is its kind's, named by its session's
  name, and serves the quest whose workspace it works in until a quest
  entry says otherwise. `src/ingest.rs` finds them; `src/series.rs` maps
  them.
- **Waste.** Each cache write gets a cause: a compaction, a pause longer
  than the cache lives, a fresh session, a model change. Rebuilt context is
  the money a session spent having again what it had. Some is how an
  account works, so the last seven days' is a finding only past half again
  the median share of the eight weeks before; otherwise it's a report.
- **Forecast.** A mixture of a state-space model on log daily spend and the
  same weekdays of the last eight weeks, weighted by each one's CRPS on the
  last 28 days. Picked by backtest over 175 days against the hour-of-week
  profile, which now only spreads a day across its hours. Its ranges are
  checked against what happened, not recalibrated, since spend changes faster
  than a calibration learns.
- **Caps.** A cap is a clock: simulated paths of the forecast give the
  chance of reaching it before the window rolls, and while over, when it
  comes back. Every change in allowance is history, and a cap lowered,
  lapsed, or a raise about to end is an alert. A change is told once it
  has stood ten minutes, or at once when the cap warns or refuses at it,
  and a change back within that time takes both away, since a
  provider's figure can flicker for one reading. A fall stops being an
  alert once the cap is back, and its finding clears at once, as a
  source's health does. `src/caps.rs` `held`.
- **Detectors.** Each threshold is set for a stated false-alarm rate. A
  burn is a minute CUSUM against twice the usual rate for the hour,
  alarming past the quantile eight weeks of daily maxima crossed once a
  month. It's the account running hot, so it's one finding a day, named
  for the session spending most. Costly sessions are found against their peers with empirical-Bayes
  shrinkage and Benjamini-Hochberg at 5%. Changes of regime come from
  Bayesian online changepoints, confirmed after a week. A price is a
  model's ratio of bill to list moving in a step from one level to
  another: the last three settled days agreeing with each other and all
  outside the month before's tenth to ninetieth percentiles, which lie
  within a quarter of their median, since spend another machine made
  comes and goes by the day; the estimates take the new ratio up at once.
  `src/detect.rs` holds them all, and `spend-watch backtest` replays the
  forecast, the burn and the caps on the history.
- **Sources.** A feed's source that can't be read is nothing for its first
  fifteen minutes, since a machine booting or waking fails every read until
  its network comes; then a note, or a warning when only a sign-in mends
  it. It clears on the first pass that finds it back. The history keeps
  since when each source has been as it is, so a restart doesn't reset it.
- **History.** A shape that only grows is brought up to date where it
  stands; one that doesn't read is set aside under a name nothing holds
  yet and built again, and each feed is told what's kept of its bill, none
  of it then, so it tells the bill again. One a newer spend-watch keeps,
  as when an older system boots again, is never touched: the watch holds,
  says so in the margin and the collection, and carries on once it's a
  shape it reads, or the newer one replaces it.
- **Strings.** Each finding carries a title for its row, a badge of a few
  words beside it, a note for the launcher's footer and a detail for what's
  copied and told. What asks for a look puts its note in the margin's
  footer too, which holds 28 characters, and the margin's header holds 23
  beside its name: `src/status.rs` `HEADER` and `FOOTER`.
  `src/collection.rs` lays them out as the launcher's other collections
  are: what wants a look first, then how things stand, each a figure with
  its parts a step in, then what was found. How things stand is today,
  each cap longer than a day and Earlier, so the week is said once: a
  day's cap is behind today, the week ahead behind the widest cap, and
  this week among the periods before.
- **Telling.** Notes stay in the collection. Warnings and alerts are told
  once as a notice, at most three a week outside a cap's, and withdrawn when
  they no longer hold or are marked expected. No notice holds an amount, and
  while the screen is shared the shell shows the quiet strings. Nothing is
  ever sent anywhere else.
- **Pace.** A pass costs about half a second on four months of heavy use
  (700,000 turns), because the history keeps each hour's turns summed as
  they're added, and a pass reads those rather than the turns. It runs at
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
