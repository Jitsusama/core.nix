# Your System, Now

How the whole system stands at once: every machine on the mycelium, the
mycelium itself, the home network and the code, each judged from its own
owner's answer, with the way deeper into each. The page watches the
system daemon's records live over the sites' server, and redraws a
part the moment its owner's answer moves. Nothing on it changes
anything.

## How to Read It

The page is the weather over everything Joel runs: each part is a
station on one horizon, and how it stands is its sky.

- **The places at the top:** Machines, Mycelium, Network and Code, each
  with how many things in it are worth a look. The part the cursor is
  in is underlined.
- **A headline that counts:** how many parts, how many things in them
  are worth a look, and how many parts couldn't be read, as "Four parts,
  five worth a look." Each count goes to the first row of what it
  counts. With nothing amiss, it says all clear. While it reads, it
  says it is reading the sky.
- **A lede under it:** the one thing that most wants a look, led by its
  part's name: the first item of the worst part, or why a part couldn't
  be read.
- **A standing line:** the machine it was read on, when, and whether
  what it shows is live. When the daemon or the sites' server can't be
  reached, it says since when and why, and keeps showing how things
  stood then.
- **The outlook:** the four parts as stations on one horizon, each
  wearing its sky, with its name and its forecast: how many things are
  worth a look, or its sky's name. Each goes to its part.
- **The four parts, in one column:** each has its sky in the gutter,
  its name (a link to its site), what it holds at the right ("21
  devices"), what in it is worth a look under it, and last the command
  that says more, with a copy button, and where it is in the launcher.
  The row the cursor is on shows the keys it answers.
  - Machines draws its machines, worst first, at most three, each
    linked to its own place, then how many more.
  - The others draw at most three items, the last saying how many more
    when there are more.
- **A legend at the foot:** each sky beside its name and what it means:
  Clear (the sun half risen), all good; Cloud (a cloud on the horizon),
  worth a look; Storm (a cloud with a bolt), bad; and Fog (broken
  lines), not read. A screen reader hears the meaning after each name.
- **How each part is judged:**
  - Machines: one out of reach or with a nearly or critically full disk
    is bad; one short of room, with a filling disk, or on battery at 20%
    or less is worth a look.
  - Mycelium: a link that isn't up is bad; presences left behind by
    sessions that ended are worth a look.
  - Network: a source that isn't answering is bad; anything else the
    network names as a concern, other than its ceiling, is worth a look.
  - Code: good whenever its owner answers. Trees git still lists whose
    folders are gone are agents' leftovers, said in what it holds.
  - A part whose owner can't be asked is Not read, with why.

## Keys

| Key       | Does                                                         |
| --------- | ------------------------------------------------------------ |
| j, k      | Step down or up through every part and what is in it         |
| 1 to 4    | Go to Machines, Mycelium, Network or Code                    |
| g, G      | Go to the first or the last                                  |
| Return    | Open what you're on: a part's site, or a machine's own place |
| y         | Copy the command that says more of the part you're in        |
| ?, F1     | The list of keys                                             |

## What the Data Holds

The system daemon holds three kinds of record. The `summary` has
`machine`, `read_at` (RFC 3339, UTC), `health` (good, watch, bad or
not read), `side` (every part's, in order) and `parts`, the part names
in order. Each `part:<name>` has `part` (machines, mycelium, network or
code), `health`, `says`, `side` (what is worth a look, an item each),
`holds` (what it holds, or null when it wasn't read), `things` (the
keys of its things) and `deeper`: `command`, `site` (from the sites'
root) and `launcher` (its path there). Each `machine:<name>` has
`name`, `health`, `says` and, where its home publishes one, `address`.
`system --json` says the same as one record.

The system asks each owner rather than reading their sources:
`mycelium machines`, `mycelium health`, `net --json` and `repos --json`,
at once, each given ten seconds.
