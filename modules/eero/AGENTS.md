# eero

The home network's eero, read from eero's cloud, since the eero has no
local API. A Rust program, `eero`, signs in once (`src/session.rs` seals
the session), reads, and changes one setting, Optimize for Conferencing
and Gaming. Joel's bar for it (2026-11-07): "i want insanely nerdy
integration with the eero": every figure eero knows that is true and
useful, shown where he looks, never a summary that hides them.

## What It Reads

[`contract/`][contract] holds every answer the eero app's reads give,
from a real three-node network, redacted, and its README says what each
figure means and where it shows, or why it doesn't. A figure is shown
only once the contract has it and the README places it.

## The Model

[`home-network.json`][model] is the home network as one record: its
internet, its nodes with their backhaul, radios and ports, its devices
with their band, signal, rates and speed, its profiles and what
happened. It's drawn from the contract's files, so it's the parsers'
golden answer, and it's the shape the shell and the CLI read from
`$XDG_RUNTIME_DIR/eero/home-network.json`. Its words are the
glossary's Network words; a figure keeps its unit in its key
(`signal_dbm`, `busy_pct`, `up_s`), and one eero doesn't give is `null`,
never a guess.

eero gives each figure only as it stands, so what moves is the daemon's
to keep, and the model carries it as a `history` beside the figure: the
last hour as a series, one list a figure beside its `at`, a sample a
read, oldest first, so a spark reads one list. Its golden answer is the
contract read through a short history `src/golden.rs` builds: six reads
five minutes apart, a roam, a join, a leave and an outage the probe saw,
on a machine that's the first laptop.

- `read`, `cloud`: when eero last answered, and whether it's answering,
  unreachable or signed out.
- `internet`: the home network's way out, its settings, the last speed
  test and every one eero kept or the history kept from the last 30
  days, firmware, outages and the day's traffic; `probe`, this machine's
  own last look at the internet; and `history`, the last hour's looks
  and 30 days of outages, as eero recorded them or the probe saw them.
- `nodes[]`: each eero by `id`, its `backhaul` (Ethernet, or the
  `upstream` node's location and radio over Wi-Fi), each radio's band,
  channel, width, power, how busy and its devices, each port's speed,
  and the day's traffic; the last hour of its bars, and of each radio's
  channel, how busy and power.
- `devices[]`: each device on now, by `mac`, whether it's
  `this_machine` (one of the daemon's machine's own hardware addresses
  is its), its `node`, band, channel,
  signal, SNR, each direction's rate with its MCS, streams, width, guard
  interval and Wi-Fi generation, retries, packets, and its speed now;
  the last hour of its signal, SNR, rates and speed, and its `roams` to
  another node or band over the last day.
- `profiles[]`: each profile by `id`, its devices, whether it's held
  off, its schedules, what its `filters` block and let through, and its
  bytes by hour over the last day, where they were read (the daemon
  doesn't read them yet).
- `events[]`: the app's activity list.
- `joins_and_leaves[]`: the devices that joined or left over the last
  day, with the name each had.
- `told[]`: what the daemon told as notices over the last 30 days,
  newest first and at most 50, each with when, its kind and the notice's
  summary and body; an event it told from eero's activity list is left
  out, since `events[]` holds it in eero's words.

## How It Reads

`eero daemon` (the `eero-daemon` user unit) keeps the model, drawn by
[`src/read/`][read] from each read's answer and written whole, and
only when it changes. It reads the account's first network, each part
as often as its figures move ([`src/schedule.rs`][schedule]):

| Read                                             | Requests        | Every                        |
| ------------------------------------------------ | --------------- | ---------------------------- |
| the network and its devices                      | 2               | 30 s listened to, else 5 min |
| each node's connections                          | 1 a node        | 5 min                        |
| the day's traffic, the network's and each node's | 1, and 1 a node | 15 min                       |
| speed tests, profiles, Thread, outages, events   | 5               | 15 min                       |

Its socket, `$XDG_RUNTIME_DIR/eero/socket`, speaks the daemon interface
the shell holds every daemon's on ([`src/listeners.rs`][listeners]): the
model is one record, kind and key `home-network`, under `network`, and a
watch is answered with it and told every change. The shell holds it from
its start. While anyone watches it ahead, as the Home Network and
`eero watch` do while shown, the network is read at the fast pace, and
an ask `read` asks for it now, at most once every 5 seconds.

After each read the daemon records what moves in
`$XDG_STATE_HOME/eero/history.sqlite`, beside the sealed session, and
draws it into each model it writes ([`src/history/`][history]): each
device's signal, SNR, rates and speed, each node's bars and each
radio's channel, how busy and power, a sample a read; a device joining,
leaving or roaming, recorded only between reads at most 10 minutes
apart, since past that nobody saw when; every speed test; and every
outage, eero's and the probe's. It keeps 30 days, a sample at most
every 25 seconds, so asked reads can't grow it past the 30-second pace.
Read at that pace all month, 24 devices with every figure come to about
117 MB, and a busy hour, 32 devices, makes the model about 336 kB.

With each read of the network, and every 10 seconds while eero can't be
reached, the daemon takes its own look at the internet
([`src/probe.rs`][probe]), since eero's cloud is across the internet it
reports on: a TCP connect to the gateway node's DNS port and to 1.1.1.1
and 8.8.8.8 on 443, and a DNS query of its own through the gateway
node, all at once, each tried three times in a row within its 2
seconds and timed by the middle try, so one packet late on the way
isn't read as a slow line; a try not answered ends its tries, so an
outage still costs one timeout. Connects, not pings: a
raw socket needs root, and a ping socket only the groups
`net.ipv4.ping_group_range` names. What this machine sees is the home
network's internet only from home, so the gateway is timed, and an
outage recorded, only while eero lists one of this machine's hardware
addresses and a route reaches the gateway node straight. An outage runs
from the first look that finds the internet out to the first that finds
it back, or to the last that found it out when nothing looked for over
10 minutes between.

After each look the daemon rewrites the day's figures whole in
`$XDG_STATE_HOME/eero/day/<date>.json`, the date in the machine's zone
([`src/day.rs`][day]), for a review to hold the home network to: how
many looks came from home and from elsewhere, the 95th percentile of
the gateway's, the internet's and DNS's times from home, how many found
the internet out, and the minutes it was out. Only looks from home
count. They are idle-time figures, not under load; loss and delay under
load are the shaper's. A day's file goes once the history no longer
keeps the day.

The model's `cloud.state` says whether the figures are current:

- `answering`: eero answered the last read.
- `unreachable`: eero couldn't be reached or failed; the figures are the
  last it gave, `since` says from when and `reason` why. Once it answers
  again, every part is read at once.
- `signed-out`: the session aged past refreshing; `reason` says to run
  `eero sign-in`, and the next read opens the newly sealed session.

## What It Tells

The daemon tells Joel what changed on the home network as desktop
notices, signed "Home network", on the session bus's
`org.freedesktop.Notifications` as disk-guard raises its own
([`src/desktop.rs`][desktop]). Which changes, and their words, are a pure
function of the last model it compared and the new one
([`src/notices.rs`][notices]), tested over `home-network.json` with an
edit a case:

- the internet out, as eero says while it answers or as the probe finds
  it from home, critical; and back, saying how long it was out;
- a node offline, by its location, with its devices and backhaul,
  critical; and back online, saying how long it was offline;
- a device eero first saw since its last answer, by its name, its node
  and Ethernet or Wi-Fi; a device coming back says nothing;
- a firmware update waiting, and the nodes' firmware changing, one
  notice for every node that changed to the same firmware;
- an event eero adds to its activity list that says none of these: one
  about a device, or naming a node or the internet going offline, online
  or updating, is told by its change instead.

Only what changes while the daemon watches is told: until eero first
answers after a start the figures are the last run's, and the first model
it answers says nothing, setting what the next is compared with. Each
change told is recorded in the history's `told` table by its kind and
subject, such as a node's id and its last heartbeat, so a restart, or a
figure flapping back, never tells it twice, and an outage told out is
timed from when it was told. Each keeps its notice's summary and body
too, which the model draws as `told`, one a notice however many changes
it told; a row recorded before notices kept their words has none, and
isn't drawn. A notice about the internet, a node or the
firmware takes the place of the last one about it, so "The internet is
back" replaces "The internet is out".

## The Views

The command line's views (`src/views/`) read the record and never eero's
cloud, so each is instant and opens with how old what it shows is, and
says so when eero isn't answering or the daemon has stopped renewing it.
Each is a pure function of the record, the time and the ink, set as text
for a person or, with `--json`, as the record's own slice of it, which
names its schema, `eero.<view>/1`, first. The command line is on the
terminal kit (docs/commands.md): it gives the help, the shared flags,
whether to ink and the exits, 69 when the daemon isn't keeping the
record, 77 when eero needs signing in and 66 for a name the network
hasn't got. The ink is still eero's own (`src/views/theme.rs`), set by
the kit's choice; drawing it with the kit's pen waits.
`eero watch <view>` ([`src/watch.rs`][watch]) listens on the socket and
draws the view again with each model, and when its reading's age next
reads differently. A page fits the terminal it's set in, as
docs/commands.md has it, and into a pipe stays whole: a view says what
it holds as lines, facts and tables (`src/views/text.rs`), and the page
is laid out in the room it's given. Lines and values wrap at their items,
then their words, under where they started (`src/views/fit.rs`); a table
gives way by whole columns in the order it names, least needed first,
then shortens its names in their middle and cuts or wraps its last
column. Every page is held to a golden one drawn from
`home-network.json` (`src/views/golden/`), into a pipe and at 60, 80, 100
and 140 columns: change a page on purpose with
`EERO_GOLDEN=write cargo test`, and read the pages it wrote before
committing them, and put back `tests/rows.json`, which it writes
formatted otherwise. A view shows every figure its part of the record has,
figures lined up by their point with their units beside them, each
measure in three significant digits and the unit that reads it
(`src/views/figures.rs`, as docs/design.md's Set, Not Styled has it), a
count saying what it counts ("24 known"), and colour
only on a terminal, in the theme's terminal colour for each role
(`src/views/theme.rs`): text in the text roles, and alert and warning
only on the marks beside it. The record keeps eero's word, usage, for the
bytes moved; every page says traffic, the glossary's word.

zsh completes eero with what `eero completion zsh` writes
([`src/completion.zsh`][completion]), which asks `eero names <kind>` for
the views and for the nodes, devices and profiles the record holds now,
each by the name its view finds it by: a device two share a name with is
offered by its hardware address.

[contract]: contract/README.md
[model]: home-network.json
[read]: src/read/mod.rs
[schedule]: src/schedule.rs
[listeners]: src/listeners.rs
[watch]: src/watch.rs
[completion]: src/completion.zsh

[history]: src/history/mod.rs
[probe]: src/probe.rs
[day]: src/day.rs
[notices]: src/notices.rs
[desktop]: src/desktop.rs
