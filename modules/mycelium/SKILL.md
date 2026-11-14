---
name: mycelium
description: >
  What the mycelium is doing: every session and machine as cells, and the
  networks they share, read from the node. Use when asked what the agents,
  sessions, swarm, teams, quests or machines are doing, what needs Joel,
  what's stuck, where the work is, what a network or a session is up to,
  or which machine is busy.
---

# The Mycelium

The mycelium is a realm's sessions and machines, each a cell, and the
networks they share. A cell joins a network by carrying its shape: a
session joins the networks of the work it does, and the machines share a
network of their own. The node on each machine keeps it, and every view
draws the same model: the launcher, the map page, the command line and
you. So answer from the node, never from memory or a guess.

## Reading It

Both commands read the realm in `MYCELIUM_REALM`; `--realm` names another.
Add `-o json` for JSON lines: networks first, then cells.

```sh
mycelium networks -o json          # every network, then every cell
mycelium networks grow -o json     # one network, by name, short name or title words
mycelium cells -o json             # every session worst first, then the machines
mycelium cells QQ1UAK -o json      # one cell and the networks it joined
```

A network line has `name`, `short` (what a person types), `title`,
`shape` (`orchestration`, `project`, `repository`, `group` or
`machines`), `within` (the network it branched from), `cells`,
`machines`, and `counts`: how many of its sessions stand as each of
`needs-you`, `stuck`, `moving`, `quiet` and `dormant`. A session cell has
`name` (its address), `title`, `standing` (one of those five), `doing`,
`machine`, `quest` and `networks`; a machine cell has `state` (such as
`ready`), `busy` (the share of the last ten seconds some task waited on a
processor) and `runs` (how many sessions it runs).

Where `mycelium networks` isn't known yet, the machine hasn't updated onto
it; `mycelium swarm` gives the teams and their sessions instead.

## Answering

Answer in words, short, worst first, the way the person would scan it:

1. What needs him: each cell standing `needs-you`, by its title, with what
   it's `doing`. Say plainly when nothing does.
2. What's stuck, the same way.
3. Where the work is: the networks with the most `moving` sessions, by
   title. The busiest has thickened into what the mycelium's pages call a
   cord.
4. Only when asked or unusual: dormant counts, quiet networks, a machine
   that's off or busy past half its time.

A few lines answer "what's it doing?"; one network or one cell gets a few
more. Name networks by title or short name and cells by title, never by a
bare address or a quest id unless asked. Never paste the JSON; count and
summarise it. Say the time you read it, since it moves.

## What You Don't Do

These commands only read. Speaking, pausing or resuming through `mycelium
swarm` acts as the person, so do it only when he asked for that act in this
conversation, and say what you did.
