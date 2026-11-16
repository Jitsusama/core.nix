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
system          # the same, as a person reads it
```

The object has `schema` (system.view/1), `machine` (where it was read),
`read_at` (RFC 3339, UTC), `health` and `parts`, always in one order:
machines, mycelium, network, code. Each part has `part`, `health`, `says`
(a sentence, worst first), `things` (the machines, worst first, each with
`kind`, `name`, `health`, `says` and `address`, its home on the local
sites as the site home to it gives it, or null while none is; empty for
the other parts, for now) and `deeper`: the `command` that says more, the
`site` (from the local sites' root, at <http://sites.localhost:8484/>) and
where it is in the `launcher`.

It's small, about a kilobyte, so read it whole, or just the parts:

```sh
system --json | jq -c '{health, parts: [.parts[] | {part, health, says}]}'
```

## How Each Part Is Judged

- **Machines.** One out of reach, or with a nearly or critically full
  disk, is bad. One short of room (memory, processor, disk time), with a
  filling disk, or on battery at 20% or less, is worth a look. Another
  machine whose own health says its code is failing is bad, and one
  whose code wants a look is worth a look; its node carries that word
  in its machine record. Only code is taken from it, since the network,
  machines and mycelium are the same whole read from every machine.
- **Mycelium.** A link that isn't up is bad. Presences left by sessions
  whose processes are gone are worth a look.
- **Network.** A source that isn't answering is bad. Any other concern the
  network names, but its ceiling, is worth a look.
- **Code.** A tree git still lists whose folder is gone is worth a look.

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
