---
name: system
description: >
  How the whole system stands at once: every machine on the mycelium, the
  mycelium itself, the home network and the code, each judged by its own
  owner, with the way deeper into each. Use when asked how everything is,
  whether anything is wrong, what's worth a look, how the computer, the
  machines, the network or the repositories are doing, or where to look
  first.
---

# The System

The system is four parts read at once, each from the command that owns it,
never from its sources:

- **Machines**, every machine of the realm, from `mycelium machines`.
- **Mycelium**, its links and sessions, from `mycelium health`.
- **Network**, the home network, from `net --json`.
- **Code**, the repositories and their trees, from `repos --json`.

`system` asks all four together, each given ten seconds, and judges each
part good, watch (worth a look) or bad; a part whose owner couldn't be
asked is unread, with why. The whole takes the worst part that was read.
It keeps no daemon and no record, so its answer is always now; a run takes
under a second.

## Reading It

```sh
system --json   # system.view/1, one JSON object
system          # the same, as a person reads it, each machine's ties under it
system machine NAME          # one machine whole, and what can be done with it
system machine NAME --json   # system.machine/1, that machine's record
```

A machine's record is its thing, as in `things` below, with its `key`
(`machine:NAME`), `part`, and the `machine` and `read_at` it was read
at.

`system machine` exits 66 (not found) for a name no part lists, and 69
(unavailable) when the machines couldn't be read at all, so a node that
isn't answering is never taken for a machine that isn't there; a name is
as the node says it, case and all.

The object has `schema` (system.view/1), `machine` (where it was read),
`read_at` (RFC 3339, UTC), `health`, `side` (every part's, in order) and
`parts`, always in one order: machines, mycelium, network, code. Each
part has `part`, `health`, `says` (a sentence, worst first), `side` (what
is worth a look, an item each, worded to stand at a side: lower case
unless a name leads, no closing stop, without what the part holds, at
most three, and empty when nothing is), `holds` (what it holds, counted
in a few words, as "21 devices", or null when it wasn't read), `things`
(the machines, worst first, each with
`kind`, `name`, `health`, `says`, `address`, its home on the local
sites as the site home to it gives it, or null while none is, and
`ties`, what ties it to the other parts: `cells` (busy, idle and ended,
all 0 where the health lists none on it, or null when the mycelium's
health wasn't read), `network` (a place for each device the network
names for it, whatever the name's case: `by` its band or `wired`, the
`point` it reaches through, `bars`, `room_mbps` in whole Mb/s and the
`device` id; null when the network wasn't read), `code` (repositories,
checkouts, trees, checkouts behind and open changes, for this machine
only, since repos reads only its own; null for another), `figures` as
the node says them, bytes to the gigabyte and `cpu_busy` to the tenth,
so a record moves only when what it says moves, and `acts`, the
launcher `link` and the `commands`, or null for a name mycelium-link
wouldn't take;
empty for the other parts, for now) and `deeper`: the `command` that says more, the
`site` (from the local sites' root, at <http://sites.localhost:8484/>) and
where it is in the `launcher`.

It's small, a few kilobytes, so read it whole, or just the parts:

```sh
system --json | jq -c '{health, parts: [.parts[] | {part, health, says}]}'
```

## How Each Part Is Judged

- **Machines.** One out of reach, or with a nearly or critically full
  disk, is bad. One with a filling disk, or on battery at 20% or less
  off the charger, is worth a look, and so is this machine for a day
  after a boot whose firmware kept an error from before it, as an abrupt
  reset leaves ("booted 3 hours ago after a firmware error"), read from
  the kernel's count in this boot's journal and the uptime. Being short
  of room (memory, processor, disk time) is said but never worth a look:
  agents' builds hold the processor nearly all day, which the person
  can't act on, and memory waits for a day measured to tell a held shortage from a spike.
  Each machine's record carries its own `side`, worded for a row ("filling
  disk, 40 GB free"), its worst first. Another
  machine whose own health says its code is failing is bad, and one
  whose code wants a look is worth a look; its node carries that word
  in its machine record. Only code is taken from it, since the network,
  machines and mycelium are the same whole read from every machine.
  The machine `system` runs on also says its quest backup once the time
  it last backed up (its time file, `quest-backup/landed` under the
  state home, epoch seconds) is over 2 hours old, worth a look, or 7
  whole hours or more, bad, said in whole hours. A time file that can't
  be read, says no time (0 or less included), or says one more than a
  minute ahead of the clock is worth a look, so a broken writer isn't
  quiet; no file says nothing, as on a machine keeping no backup, and so
  does a backup that has never pushed anything. Only that machine's
  backup is read, since the file is its own; from another machine it
  says nothing of it. When the machines can't be
  read, the machines part says a late backup itself. The backup runs
  hourly and catches up on waking, so a laptop waking from a night reads
  late for the minute or two that run takes.
- **Mycelium.** A link that isn't up is bad. A session whose process is
  gone is an agent's leftover, said in what the part holds ("3 sessions
  gone") and never worth a look.
- **Network.** A source that isn't answering is bad. Any other concern the
  network names, but its ceiling, is worth a look.
- **Code.** Good whenever `repos` answers. A tree git still lists whose
  folder is gone is an agent's leftover, which the work tool's tidy
  clears and the person can't act on from the code, so it is said in
  what the part holds ("3 with their folders gone") and is never worth
  a look.

## Answering

Answer in words, short, worst first, as the person would scan it:

1. The whole in one sentence: what's bad, then what's worth a look, or
   that every part is good.
2. Each part that isn't good, with what it `says`.
3. Where to go deeper, only for what he'd act on: the part's `command`
   to run, or its site, or a thing's `address` under the sites' root. Use
   the address as given; never build one by hand.

Then go deeper with the part's own command or skill, never by reading its
sources: `mycelium` for the machines and the sessions, `net` for the
network, `repos` for the code. A part that's unread says why in `says`;
pass that on rather than guessing at the part.
