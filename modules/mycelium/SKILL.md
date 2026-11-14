---
name: mycelium
description: >
  What the mycelium is doing: every session and machine as cells, and the
  networks they share, read from the node. Use when asked what the agents,
  sessions, swarm, teams, quests or machines are doing, what needs Joel,
  what's stuck, where the work is, what a network or a session is up to,
  which machine is busy, or which sessions cost most today.
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
mycelium cells NAME -o json        # one cell, by name or title words, and its networks
```

A network line has `name`, `short` (what a person types), `title`,
`shape` (`orchestration`, `project`, `repository`, `group` or
`machines`), `within` (the network it branched from), `cells`,
`machines`, and `counts`: how many of its sessions stand as each of
`needs-you`, `stuck`, `moving`, `quiet` and `dormant`. A session cell has
`name` (its address), `title`, `standing` (one of those five), `doing`,
`asks` (what it asks the person, in its own words), `repo`, `machine`,
`quest`, `networks`, `leads` (the networks it leads) and `usage`:
`cost_today` in US dollars, `context` as `{used, limit}`, `turns_today`
and `last_turn`. A machine cell has `state` (such as `ready`), `busy` (a
percentage from 0 to 100 of the last ten seconds some task waited on a
processor; past 50 is busy) and `runs` (how many sessions it runs).

## Answering

Answer in words, short, worst first, the way the person would scan it:

1. What needs him: each cell standing `needs-you`, by its title, with what
   it `asks` (or, without one, what it's `doing`). Say plainly when
   nothing does.
2. What's stuck, the same way.
3. Where the work is: the networks with the most `moving` sessions, by
   title. The busiest has thickened into what the mycelium's pages call a
   cord.
4. Only when asked or unusual: dormant counts, quiet networks, a machine
   that's off or busy past 50.

Asked what's costing most, name the few sessions with the highest
`usage.cost_today`, in dollars, with the networks they work in. A session
can join several networks, so a sum by network counts it more than once:
say so if you give one.

A few lines answer "what's it doing?"; one network or one cell gets a few
more. Name networks by title or short name and cells by title, never by a
bare address or a quest id unless asked. Never paste the JSON; count and
summarise it. Say the time you read it, since it moves.

## What You Don't Do

Use only `mycelium networks` and `mycelium cells`, which only read. Never
run `send`, `steer`, `start`, `holds` or `carry` to answer a question:
they act on sessions and machines, and some act as the person.
