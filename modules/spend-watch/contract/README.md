# The Feed Contract

A feed brings in what a provider bills, for a machine whose provider can say.
spend-watch starts each feed named in `jitsusama.spend-watch.feeds`, reads
JSON lines from its stdout, writes what it keeps and the pace on its stdin,
and restarts it when it stops: after 5 seconds, doubling to 5 minutes while
it keeps stopping within a minute. What the feed writes on stderr goes to
the journal. A feed that doesn't read its stdin loses the lines it couldn't
take rather than stopping the daemon.

Nothing a feed is started with may hold a secret: its command and
environment are in the Nix store, which anyone on the machine reads. A feed
fetches its credentials itself.

Every line either way is one JSON object with `"v": 1`. A line of another
version, or one that doesn't read, is put in the journal and skipped, so a
feed newer than its daemon degrades one record at a time. The examples beside
this are what both sides test against: `spend-watch contract check <file>`
reads a feed's output and says which lines don't hold.

## What spend-watch Writes

A feed passes over a line it doesn't know, so one added later is news only
to the feeds that read it.

- `{"v": 1, "pace": "active"}`: pi's logs grew in the last ten minutes, so
  the bill is worth reading often; `idle` when they haven't. One is written
  when the feed starts and on every change. `asleep` is reserved: a
  suspended machine's feed is frozen with it, so nothing needs saying. A
  feed reads every pace it doesn't know as `idle`.
- `{"v": 1, "kept": {"billed": {"warehouse": "2026-10-14T13:00:00Z"},
  "caps": {"weekly": "2026-10-14T16:04:29Z"}}}`: what the watch keeps of
  this feed's, written once as it starts, after the pace: for each source,
  when the newest hour of its bill starts, and for each cap, when its
  newest reading was taken. A source or cap left out has nothing kept. A
  feed that keeps how far it has read starts again no later than this, so
  a history built afresh is told the bill again, and one that keeps
  nothing reads from its own start.
- `{"v": 1, "refresh": true}`: the person asked for the latest figures now.
  `spend-watch refresh` answers once every running feed has written a line
  since, or after ten seconds, so a feed should read at once and say what it
  found, a `read` record at least.

## What a Feed Writes

A feed speaks for one budget, the money a request draws on: its own,
`feeds.<name>.budget`, or the default budget when that's unset. A `billed`
or `cap` record for another budget names it in `budget`.

Instants are RFC 3339; days are `YYYY-MM-DD`. Each record has a `kind`:

### billed

Dollars billed in one hour, by one source, for one key: the source, the
hour, the tool, the model and, when given, the session, agent and token. A
later record for the same key replaces an earlier one, so a feed may say an
hour again as often as it likes.

- `source`: `warehouse` for the bill as it settles, `meter` for a live
  figure. Two sources never add up: the warehouse's settled hours win, the
  meter fills the hours it hasn't settled, and pi's logs fill the rest.
- `hour`: the instant the hour starts.
- `tool`, `model`, `vendor`: what spent it. `tool` is `all` when the source
  can't tell.
- `session`, `agent`, `token`: optional. A session lets the hour be split
  by what it served; a token never seen before is a finding, so a feed
  whose provider mints credentials as a matter of course leaves it out.
  A token is an opaque name for the credential, never the secret itself.
- `budget`: optional, the budget it was drawn from, by the name
  `jitsusama.spend-watch.budgets` declares it; absent means the feed's
  own (`feeds.<name>.budget`, or the default budget). A figure that
  doesn't name its session is shared only among its budget's pieces, so
  a feed that can tell a team's token from the person's own key says so
  here.
- A key is the source, hour, tool, model, vendor, session, agent, token
  and budget: a feed says each hour's figure for a key once, summed,
  since a second record for it replaces the first.
- `tokens`: optional `input`, `cached`, `cacheWrite`, `output`.
- `dollars`: a figure of none or more.
- `provisional`: the hour is still open and this is what it stands at.

### settled

`source` has billed everything before `through`, an hour boundary. Hours a
source has settled are its exact bill; hours it hasn't, it may still change.

### cap

An allowance and where spend stands against it, as of `at`: `name`, `label`,
`allowance` in dollars, more than nothing (null for none), `windowHours`, `spent`, `warn` as a
fraction of the allowance, and `state`: `ok`, `notified` or `refused`.
`fallback` is the allowance that applies when a raised one ends, on `ends`;
`endsKnown` says whether the feed could know the end at all. Every change in
allowance or state is kept as the cap's history.

`label` is the window's name, as the launcher shows it and a sentence says
it ("near the Premium Day allowance"): its length in words, in Title Case,
with what it's for in front when it covers only part of the budget:
"Five Hours", "Week", "Premium Day". It never opens with a figure and
never says cap, limit or quota, since the glossary's words are budget,
allowance and window. A figure says the window short itself ("62% of
5 h"), whatever the label. A cap whose label breaks this is refused, as
any record that breaks the contract is.

`budget` is optional, as a bill's is: the budget it caps, the feed's own
when absent. A cap's outlook, chances and warnings count only its budget's
spend, and a feed may give two budgets' caps of the same name.

`unit` is `dollars` when absent, or `share`, for a plan's window
(Claude's five hours, its week) whose use is a share: a share gives `used`,
a fraction from 0 to 1, and `resets`, in place of `allowance`, `spent` and
`fallback`. A share is never turned into dollars, so it has no forecast in
dollars; `warn` and `state` still apply, and once it resets with no newer
reading it stands at none used.

A cap counts a rolling window of `windowHours` unless it gives `resets`, an
instant after `at`: then it counts periods of `windowHours`, such as a
calendar day, and its spend goes all at once when one ends. The watch keeps
resetting it every `windowHours` after, so a feed need only say it again
when the reset moves.

### action

Something the person can do, offered beside the finding it `fits`: `cap`,
`sign-in` or `premium`. `run` is the program and its arguments.

### health

How one source is: `fine`, `stalled`, `signed-out`, `optional` (a source
not signed in for, which is no fault) or `error`, with a `message` and the
`signIn` action that mends it. A source that's down says so here and the
figures fall back a step; nothing else stops. One down for less than a
quarter of an hour isn't told, so a feed should say each read's state as
it is, without holding back a failure it expects to pass.

### read

What one read of a source cost, in `bytes` and `millis`, so the cost of
watching is seen beside what it watches.

### said

Something the provider said to the person, with its `severity`, shown
among the findings for a quarter of an hour after it's said: `critical`,
`error`, `alert` and `warning` as a warning, anything else as a note.
Its `text` is plain words; markup is drawn as written, angle brackets
left out.
