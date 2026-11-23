# spend-watch

A Rust daemon that keeps the account told what its AI use costs. It reads
pi's session logs as they grow, its agents' as well (a swarm's, a review
council's), the harness's subagent runs and its quests, and the feeds a
machine names for a provider's bill; keeps every dollar by
hour in `history.db`; and writes `status.json` and `collection.json` for the
shell, the overview's row, the margin and the collection on Super+U, and
`budgets.json` for pi's footer, a budget's drawer in the launcher and the
budget a session's drawer names. Every fact in it is in words too, each
budget's and allowance's `said`, so neither pi nor the shell formats one. A
cap's row in `collection.json` names its window there, so both open one
drawer, and a period's row carries its own drawer's facts, `drawn`. Its
socket tells the shell the same collection and budgets as records, the
shell's daemon interface (`src/interface.rs`): a session's figures, a
quest's, a row, a report and its words, a budget and the budget a session
drew on each one record, and after each pass only those that changed, so
the shell remakes only what a pass moved and watches each kind only while
it draws it. The launcher's rows are told ready too (`src/ready.rs`), in
the form every daemon serves its rows in (`../ready-rows`): each worded
from the launcher's words.json, which the config names as `vocabulary`,
and spaced, so the launcher only places and draws them. A watch naming a
`place` hears only that place's rows and summary, so the shown list's
reader and the quiet one's each hold their own. The socket takes
the shell's acts too (`src/acts.rs`): `run` does what a row offers, going
to a session or starting its sign-in through niri, and `expect` marks its
finding expected, each named by the row's key so the socket runs only what
a pass put in a row. The bar's piece acts the same way, naming an action
of `status.json` by its `key`. A report's row carries no copy of its own:
its words change with each of its figures, so carrying them re-sent the
row each pass, and the launcher asks `copy` with the report's id at
Return instead, answered from what's held. The files stay for pi, the command line and
the checks, and for the shell while the socket isn't there. A check or a
VM test standing in for the daemon writes those files and runs
`spend-watch serve`, which tells them on the socket as a pass would,
answers a report's copy, and refuses every act as not answering.
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
  tool, agent kind and budget, so every cut is a sum of the same pieces and
  every report agrees with every other. A quest's spend rolls up to its root,
  and for the launcher's Agents to every quest above it, today's each a
  family in `collection.json` (its whole, its own and its subquests'), so
  an orchestrator says its orchestration's without the shell adding. A
  bill that doesn't say which session spent it is shared by its budget's
  pieces on its model, by their list prices, since models are priced
  apart; a metered hour keeps what other tools spent on each model as
  theirs on it.
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
  entry says otherwise. A folder at a pattern's first `*` (a council's run,
  a quest's workspace) that has gone a day unwritten, its logs read to
  their end, is walked no more, though its logs are still read each pass.
  It's walked again when a folder the walk read in it changes (a reviewer
  asked again makes one), when the watch hears a write under it, and at
  least every six hours, which finds a log made in a folder that held none.
  One whose log folder the watch can't follow, as when the account's
  inotify watches are spent, is walked each pass, said once in the journal.
  `src/ingest.rs` finds them; `src/series.rs` maps them.
- **Machines.** Another machine is counted as this one is, the only
  difference being where its credentials live: the config names where its
  session logs and its agents' logs are brought, and each is read the
  same way, so its budgets come from its own logs' entries. Each session
  is its machine's, kept on the session rather than each turn, since a
  run's agents run where their session does; a session read before
  machines were is this machine's. The by-machine cut shows each machine
  by its name, this one by the config's `machine` or else its host name,
  pi spend no log explains as elsewhere, and another tool's bill as its
  tool's, since a bill names no machine. It shows only for a period in
  which another machine spent money, not just took turns on a plan. Only
  another machine's logs are read: its subagent runs and quest
  workspaces stay there, so its runs' spend beyond their logs is
  elsewhere. A copy's turns from before its header are its parent's when
  the parent's log is beside it or any of the parent's turns from before
  the fork are held already, read from any machine's logs, so every copy of a
  session agrees, and a parent the history knows only by its header takes
  nothing; a copy brought before its parent counts whole until the parent
  lands, then is read again for its own turns. A brought log drops what a session of
  this machine's was counted for only when its header begins when this
  machine's log of it does. Each machine's sessions folder
  stands apart from every other's where it really is, links passed over
  inside, so nothing is counted twice. A session copied to another
  machine and gone on there keeps its id, so its logs share the turns
  from before the copy: each turn is kept once by its session, time,
  model, kind and tokens, and each refusal and unresolved budget by its
  session, time and model or budget, whichever log said it first. A
  session once read from a log here stays this machine's, as this
  machine says it; one only elsewhere is the machine's whose log was
  last read from its start, so a machine renamed, or its logs brought
  somewhere new, takes its sessions along. A log replaced or shrunk is
  read again from its start. When it's its session's only log, what it
  no longer says is no longer counted; when it's one of several, gone
  from disk or not, its session's turns stay, since which log said what
  isn't kept, and clearing old logs away never lowers what was counted. A finding
  on another machine's session offers no terminal to go to. The logs
  are read as they grow in place, which whatever brings them has to
  keep to: a log replaced on each copy is read again whole every time,
  and a session two other machines both carry on then takes the machine
  of whichever log was read last.
  A brought folder is trusted as fully as
  this machine's own: its refusals, budgets and workspaces count here,
  and its headers can drop what another brought machine's logs were
  counted for; only this machine's counts are guarded. A copy cleared
  away while it still waits on its parent keeps the parent's turns.
- **Budgets.** pi's logs name one provider for everything a proxy
  carries, whether it spends Joel's own key or a team's token, so each
  turn's budget is worked out: the budget claiming its provider, since
  that's what's paid (pi-claude's `claude-code` is the Claude plan's);
  else the one the nearest budget entry among its ancestors names, which
  the harness writes as a session's budget changes; else the default. A
  run's agent whose log names none draws on what its session did at that
  hour. A name nobody declared counts as itself, shown as undeclared, so
  the budgets always sum to the whole. A feed's bill and caps are its
  budget's unless a record names another; a budget no feed bills is
  estimated from the logs; a plan's turns cost nothing but its price,
  their list prices kept as what they were worth. Each allowance
  counts only its budget's spend, and so do its outlook, its warnings and
  the tokens warning. A share of a window is never turned into dollars,
  so the whole across budgets is billed dollars only: metered spend and
  each plan's price over the days it covers. `src/budgets.rs`
  attributes, `src/standing.rs` says how each stands.
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
- **Caps.** A cap is a clock: simulated paths of the forecast, at its
  budget's share of all spend, give the chance of reaching it before the
  window rolls, and while over, when it comes back. Every change in
  allowance is history, and a cap lowered, lapsed, or a raise about to
  end is an alert. A change is told once it
  has stood ten minutes, or at once when the cap warns or refuses at it,
  and a change back within that time takes both away, since a
  provider's figure can flicker for one reading. A fall stops being an
  alert once the cap is back, and its finding clears at once, as a
  source's health does. `src/caps.rs` `held`. A raise whose end the feed
  can't say, over a fallback already spent, is warned while it stands,
  since the cap refuses the moment it ends and no day can be warned
  ahead of; one the feed could know and gives no end isn't, since it has
  none. The clock's middle path says in which hour the allowance is
  reached at this pace, over the hours its paths cover, said by the
  hour's start so a warning is never late, and the warning
  and the near one carry it with the hour the pace was judged in.
- **Detectors.** Each threshold is set for a stated false-alarm rate. A
  burn is a minute CUSUM against twice a working hour's rate (the median
  hour of eight weeks that spent a dollar) at any time of day, since
  agents left working overnight are work, found past the quantile eight
  weeks of daily maxima crossed once a month. It's the account running
  hot, so it's one note a day, named for the session spending most, and
  never told: the caps say when spending fast is about to cost access.
  Costly sessions are found against their peers with empirical-Bayes
  shrinkage and Benjamini-Hochberg at 5%. Changes of regime come from
  Bayesian online changepoints, confirmed after a week. Something new
  billed in the last day is a note when it's a model or tool, which
  someone chose, and told when it's a credential, which may be a leaked
  key; a day leaning on one model isn't a finding, since a switch to a
  newer one is all it ever said. A price is a
  model's ratio of bill to list moving in a step from one level to
  another: the last three settled days agreeing with each other and all
  outside the month before's tenth to ninetieth percentiles, which lie
  within a quarter of their median, since spend another machine made
  comes and goes by the day; the estimates take the new ratio up at once.
  A budget a feed billed pi for before the day, whose key the warehouse
  billed pi under half what its sessions logged over the last 24 hours
  every feed the config runs has settled, by at least the floor, is
  told: its work went on another key, as two teams' did on 17 and 18
  November when the pi host sent its own key for every conversation.
  Plans and budgets first billed within the day are left out, since
  nothing bills them to disagree; a feed settled more than 48 hours
  behind, or past now, is told once in their place, by name, and a
  feed the config runs that bills pi without saying how far it has
  settled holds every budget back, told as a note under a key of its
  own, after any wrong row, since it hides none. Every such feed is
  told, by name, and once its note has stood for more than 48 hours a
  warning under a key of its own stands beside it, so marking the note
  expected leaves the warning. A budget a session asked for in the last
  day and the harness couldn't draw on, as its budget entry says, is
  told with how many sessions missed it, the day they start rather
  than a day later. Cadence counts both kinds of finding each hour
  while spend-watch's heartbeat is current.
  `src/detect.rs` holds them all, and `spend-watch backtest` replays the
  forecast, the burn and the caps on the history.
- **Sources.** A feed's source that can't be read is nothing for its first
  fifteen minutes, since a machine booting or waking fails every read until
  its network comes; then a note, or a warning when only a sign-in mends
  it. It clears on the first pass that finds it back. The history keeps
  since when each source has been as it is, so a restart doesn't reset it.
- **History.** A shape that only grows is brought up to date where it
  stands, by the daemon as it starts, in minutes on a large one; a
  command reading it meanwhile is told to try again, and with no daemon
  it moves the history itself under the lock. One that doesn't read is
  set aside under a name nothing holds
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
  each cap longer than a day, the budgets where there's more than one,
  then each period before, Yesterday to the last 30 days: a day's cap is
  behind today, the week ahead behind the widest cap, and behind Budgets
  the whole, then each budget nearest its allowance, a budget with several
  allowances stepping in to each. The periods sit in the collection itself, so no path goes
  deeper than three (Joel, 2026-10-26). A row of the budgets names the
  theme's mark it wears, AI spend's for a budget and a clock for a window,
  and a day's row a day's; its role colours it, muted at rest. A window
  is named by its feed's label, written in Title Case at the source as
  the contract's README says, or for its length when no feed reads it
  ("Five Hours", "Week", "Premium Day"); its figure says the window short
  ("85% of 5 h"), and a sentence names it with the glossary's word after
  it, "near the Week allowance" (VKCF3Z, 2026-11-07). Each row with a
  drawer brings it ready: a period's facts, and `src/drawers.rs`'s for a
  budget, a window and a cap, its quiet row's with no dollars, so the
  shell only names each fact by its id.
- **Telling.** Notes stay in the collection. Warnings and alerts are told
  once as a notice, at most three a week outside a cap's, and withdrawn when
  they no longer hold or are marked expected. No notice holds an amount, and
  while the screen is shared the shell shows the quiet strings. Nothing is
  ever sent anywhere else.
- **Pace.** A pass costs about half a second on four months of heavy use
  (700,000 turns), because the history keeps each hour's turns summed as
  they're added, and a pass reads those rather than the turns. Finding the
  logs is the other cost: on 2026-11-22 the reviewers' pattern read 36,295
  folders for 1,283 logs, 42 seconds a walk on a busy disk and about 4 MB
  a second all day, almost all of it councils finished weeks before. So a
  settled folder is left (see Agents), and each pass hands the watch the
  folders it found rather than the watch walking again. Where inotify
  can't be had, pi's sessions folder isn't there yet as the daemon starts,
  or the watch can't be read, the daemon polls until it's restarted,
  saying so once in its journal, and each poll is a whole walk, the old
  cost. A count that looks late, or a daemon polling once pi has run, is
  mended by `systemctl --user restart spend-watch`, which forgets what's
  settled at the price of one whole walk. It runs at
  most every 30 seconds while the logs grow and every five minutes while
  they don't; the heartbeat alone is written every 30 seconds, so the shell
  can tell a stopped daemon from a quiet one. Every pi session asks
  `budgets --json` as it starts, all of them at once when an update
  reloads them, so without a session it answers with the last pass's
  `budgets.json` while that's within six minutes, rather than a pass each
  that took 20 seconds under load (2026-11-07).

## Working on It

The crate is a library with a thin `main.rs`. Its tests run as it builds;
`direnv exec . cargo test` runs them here, and `spend-watch import` followed
by `show`, `report` or `budgets` tries it on the account's own logs without
the daemon, with a config whose state lies elsewhere.

`feeds/` holds the feeds kept beside the watch, each a crate of its own
with its tests and checks: [`feeds/claude/`][3] speaks for a Claude plan,
its five hours and its week, and the module gives it to a machine as
`claudePlanFeed`.

[1]: home.nix
[2]: contract/README.md
[3]: feeds/claude/README.md
