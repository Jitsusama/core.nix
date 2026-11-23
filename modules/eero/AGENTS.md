# eero

The home network's eero, read from eero's cloud, since the eero has no
local API. A Rust program, `eero`, signs in once (`src/session.rs` seals
the session), reads, and makes the few changes What It Changes lists.
Joel's bar for it (2026-11-07): "i want insanely nerdy
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
  interval and Wi-Fi generation, retries, packets, and its speed now
  (none in a read where eero's cloud puts every device at 0.0, which it
  does nine reads in ten: that's no figure, not a home gone idle);
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
a view under `--watch` do while shown, the network is read at the fast pace, and
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

## What It Changes

The daemon makes four changes, and only these ([`src/acts/mod.rs`][acts]):
start eero's speed test, turn Optimize for Conferencing and Gaming on or
off, keep an address for a device, and stop keeping one. Each is an ask
`act` on its socket; an act it doesn't name is refused as `bad-act`, so
no DNS, port forward, firewall, admin or account change goes through
that door. The socket's folder is 0700, and the daemon answers only its
own account, by the connection's peer credentials, closing any other at
once with one line saying so, `not-ours`, so its asker knows nothing was
read. The fence gives its commands an empty runtime folder of their
own, so no fenced or strict session reaches the socket at all.
`eero devices <MAC> reserve <address>` and `eero devices <MAC> unreserve`
ask the daemon ([`src/asking.rs`][asking]) rather than opening the
session, so the session is opened by the daemon and by `eero sign-in`,
`eero internet sqm on|off` and `eero speeds run` alone.

An ask written to the socket by hand keeps its connection open both
ways until it's answered, since a listener whose side has closed
counts as gone and its act is let go; the CLI's acts do so.

The socket is the door agents are meant to use, not a wall: a process
running as the account, unfenced, can open the session itself as those
three commands do. Only the fence keeps an agent from it, by keeping
systemd's credential service out as well as the socket: `session.cred`
opens only through that service, so its ciphertext alone is no use.
The fence also binds an empty file over `session.cred`, but the daemon
renames each refreshed session into place, which may leave a fenced
shell the new file, so the line rests on the service. The fence is the
agent harness's, not this tree's, and its own tests prove what it
hides, so a change to that is made and checked there.

A reservation changes which device DHCP gives which address, so each
refuses rather than guesses ([`src/acts/reservations.rs`][reservations]). A
reservation crossing one eero keeps is refused, not replaced, and so is
an address a device or an eero holds now, or one outside the range the
home subnet gives out, where a device set up by hand may keep its own.
An unreservation is refused where a port forward may reach the
reservation, by its address, its url, its device's url or its MAC,
where eero keeps more than one for the device, where its address is
outside the range eero gives out, since keeping it again couldn't undo
it, or where its reservation lives anywhere but under the network's
reservations. No forward has
been seen, so the whole answer is searched for any of those. A list
this can't read whole, reservations, devices, eeros or forwards,
refuses too. Each change is judged by reading eero's reservations after
it rather than by eero's answer, whose shape for a change was taken
from an open-source client rather than seen. Only eero's own refusal of
a change it then doesn't keep is said refused: a send that timed out may
still land, and eero's list may lag its answer, so a change that
doesn't show yet is unknown. The CLI then exits 75, never saying it
was done or refused, and says to wait for `eero internet`, which lists
every reservation, a device off now included, to show whether it
stands: an ask again before eero's list shows it reads the same list,
so it sends the change twice and may call eero's answer a refusal.
Where the
daemon says it's unknown, an unreservation's answer says the address it
kept, which keeping again undoes it; where the CLI stops waiting
first, the act's last line in the change log holds it once the act
ends. A reservation
that already stood changed nothing, so it's said with no undo. An act
whose asker has gone before the daemon reaches it is let go undone.
The CLI's six minutes of patience leave room for a reservation behind
the daemon's longest read; a slow cloud through every read falling due
at once can take longer, and the act is then said unknown and let go,
logged `gone`, unless the daemon had begun it. The daemon reads its
reservations again as it answers, so reading them once that read ends,
a few seconds on, shows an unknown outcome unless eero's list still
lags, when the next read, fifteen minutes on, does; until that read
ends it shows the list from before the act. Where the CLI stopped
waiting, the act may still be under way, and its last line in the
change log says what it came to once it ends; where the daemon hung
up, it stopped, and the log may hold only the ask.

Each act the daemon is asked is logged as asked before it's done and
with its result after, as JSON lines in
`$XDG_STATE_HOME/eero/changes.jsonl` ([`src/acts/changes.rs`][changes]), and
one whose ask can't be logged isn't done ([`src/acts/carried.rs`][carried]).
That holds for all four acts asked of it, the speed test and
conferencing too. `eero internet sqm on|off` and `eero speeds run` open
the session themselves rather than ask the daemon, so what they change
leaves no line. One unlogged is answered `unlogged`, which the
CLI says as the daemon's own fault, exit 69, never as eero's refusal. A
line holds when, the act and the MAC and address it names, the asking
process's pid and uid from the socket's peer credentials, the unit or
scope it ran in as it connected, and its result by kind alone. An
unreservation's last line names the address eero kept for it, its undo,
whatever it came to, unless the daemon stopped between sending it and
logging its end. An act let go because its asker left is logged as
`gone`, and an ask refused as no act is logged too, its act's name cut
short: `bad-act` for an act the daemon doesn't know, which the CLI says
as a daemon older than itself, exit 69, and `bad-ask` for one it knows
asked with fields it can't read, which the CLI checks first, so it says
that as a bug, exit 70. A connection asking no act eight times is let
go, and nothing more it sent is answered, logged or done, so one asking
in a loop writes eight lines at most. One connecting again starts
again; any process of the account could fill the disk anyway, so this
bounds a mistake, not an attack. Each line is written whole in one write, on the disk before the
act goes ahead. The log holds no session and nothing eero answered, whose refusals
can carry their whole body. Nothing trims it, since it grows by a line
or two an act. It's a record the daemon keeps, not an audit: any
process running as the account can write the file, and a process can
ask through another.

The session sealed in `session.cred` is the account's alone: the file is
made mode 600 from the start and its folder 700, never narrowed after
writing, and one an older eero sealed open is narrowed as the daemon
starts. Nothing in the code waits on Joel: an agent asks for a speed
test or conferencing and gaming on its own, and asks for a reservation
or an unreservation only after Joel's yes on that one change, with what
it changes and its undo.

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
hasn't got. The ink is the kit's pen (`src/views/theme.rs` hands it
to the views), so each role's colour and each mark are the theme's as
every command draws them, and a watch draws over its last page through
the kit's screen.
Its words are a tree of resources (`src/commands.rs`), walked by the
kit (docs/commands.md, Subcommands): `eero devices Pixel` is one of a
collection, the singular an alias, `eero nodes Study devices` and
`eero profiles Kids devices` what one holds, and `eero internet sqm on`
and `eero speeds run` act on their resource, `eero internet sqm` bare
saying the setting. What one node or profile holds is the whole's view
of a record cut to it, so its JSON keeps the whole's schema; a
profile's adds `unseen`, the members eero hasn't seen lately, which
the record keeps neither on nor off. zsh's
completion asks eero what may follow the words typed
(`eero names next`, `src/completing.rs`), so it follows the tree too. The old `eero sqm`,
`eero speedtest` and `eero watch <view>` still answer, listed nowhere,
and each translates to the words `views::View` always read, so every
`eero.<view>/1` is byte for byte what it was.
A view under `--watch` ([`src/watch.rs`][watch]) listens on the socket and
draws the view again with each model, when its reading's age next
reads differently, and on a terminal when its width has changed, which
it looks at each second. A page fits the terminal it's set in, as
docs/commands.md has it, and into a pipe stays whole: a view says what
it holds as lines, facts and tables (`src/views/text.rs`), and the page
is laid out in the room it's given. Lines and values wrap at their items,
then their words, under where they started (`src/views/fit.rs`); a table
gives way by whole columns in the order it names, least needed first,
save that a column with a dash in every row goes before them all, since
eero's cloud gives most reads no device speeds. So under `--watch` a
narrow devices page shows tx on a read without speeds, and down and up
on one with them. Then it shortens its names in their middle and cuts
or wraps its last column, dropping that column instead where it'd keep fewer than six
cells. Tables a page sets as shared (`Page::shared_table`) place the
columns they lead with alike, so the devices on now and those not on
line up one under the other, and a shared column one gives way goes
from both, so a shared column counts as all dashes only where it is in
every one of them. Every page is held to a golden one drawn from
`home-network.json` (`src/views/golden/`), into a pipe and at 60, 80, 100
and 140 columns, and runs no line past 40 or 50 either, with no golden
page there: change a page on purpose with
`EERO_GOLDEN=write cargo test`, and read the pages it wrote before
committing them, and put back `tests/rows.json`, which it writes
formatted otherwise. A view shows every figure its part of the record has,
figures ending together, whole numbers and a missing figure's dash
as much as tenths, so the units after them start in one place, each
measure in three significant digits and the unit that reads it
(`src/views/figures.rs`, as docs/design.md's Set, Not Styled has it), a
count saying what it counts ("24 known"), and colour
only on a terminal, in the colour the kit's pen gives each role, its
terminal slot or, under a truecolour terminal, the pigment itself
(`src/views/text.rs`, `Ink::inked`): text in the text roles, and alert and warning
only on the marks beside it. The record keeps eero's word, usage, for the
bytes moved; every page says traffic, the glossary's word.

zsh completes eero with what `eero completion zsh` writes
([`src/completion.zsh`][completion]), which asks `eero names <kind>` for
the views and for the nodes, devices and profiles the record holds now,
each by the name its view finds it by: a device two share a name with is
offered by its hardware address.

[acts]: src/acts/mod.rs
[asking]: src/asking.rs
[carried]: src/acts/carried.rs
[changes]: src/acts/changes.rs
[reservations]: src/acts/reservations.rs
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
