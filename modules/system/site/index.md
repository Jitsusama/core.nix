# Your Machines, Now

How Joel's machines stand, and what each carries: the mycelium's cells
on it, where it meets the home network, the code it holds and its own
figures. A machine is the bridge between the mycelium, the network and
the code, so the page leads with them, and says each of those three
parts across them after. It watches the system daemon's records live
over the sites' server, and redraws the moment an owner's answer moves.
Nothing on it changes anything; what can be done goes through the
launcher or a command.

## How to Read It

Every health is good, watch, bad or not read, each with its own mark.

- **A headline that counts the machines:** how many, then how they
  stand, as "Two machines, one in trouble and one worth a look", in the
  worst one's colour. With every machine fine, it says all ready.
  While it reads, it says it is reading your machines.
- **A lede under it:** what each machine worth a look has, its name
  leading, at most three of them; then the worst thing in another part,
  or else why a part couldn't be read.
- **A standing line:** the machine it was read on, when, and whether
  what it shows is live. When the daemon or the sites' server can't be
  reached, it says since when and why, and keeps showing how things
  stood then.
- **The bridge:** the machines drawn as the piers of a bridge, each
  named above its pier with its health in words, and filled as far as
  its disk is. The mycelium's cells on each are dots along the deck,
  filled for busy and hollow for idle; past what a pier holds, one dot
  stands for several cells, and the drawing says how many. Under each
  pier, where it meets the network, as "Hall, 2.4 GHz". A wide screen
  draws four piers, a narrow one two, and the drawing says how many
  more are below. The drawing is hidden from a screen reader, since
  every word in it is said again below.
- **A section for each machine,** worst first, at most eight, then how
  many more, linked to the map. Each has its name, linked to it on the
  map, its kind (model and system, and whether it is this one) and how
  it stands, then four ties:
  - Cells: how many of the mycelium's cells are busy and idle on it,
    those ended aside, linked to it on the map.
  - Network: where it meets the network, by band or by wire, and how
    well, linked to its device on the network's page.
  - Code: what it holds, counted, linked to the code's page; said not
    read for a machine other than this one.
  - Itself: its disk, then its processor, memory, power and last
    update.

  Then what can be done with it: Open in the Launcher, which opens the
  launcher's Machines at that machine, and its commands, each with a
  copy button.
- **Across them:** the mycelium, the network and the code, each with its
  mark, its name, what it says, what in it is worth a look, a link to
  its whole page and the command that says more.
- **A legend at the foot:** each mark beside its word. The marks are
  hidden from a screen reader, which hears each machine's and each
  part's health after its name instead, as "osprey (watch)".
- **How each is judged:**
  - A machine out of reach or with a nearly or critically full disk is
    bad; one with a filling disk, or on battery at 20% or less, is
    worth a look. Being short of memory, processor or disk time is said
    but never judged, since agents' builds hold the processor nearly all
    day.
  - Mycelium: a link that isn't up is bad. Presences left behind by
    sessions that ended are agents' leftovers, said in what it holds.
  - Network: a source that isn't answering is bad; anything else the
    network names as a concern, other than its ceiling, is worth a look.
  - Code: good whenever its owner answers. Trees git still lists whose
    folders are gone are agents' leftovers, said in what it holds.
  - A part whose owner can't be asked is Not read, with why.

## Keys

| Key    | Does                                                                |
| ------ | ------------------------------------------------------------------- |
| j, k   | Step down or up through every machine and what it carries           |
| 1 to 8 | Go to a machine, in the order they're drawn                         |
| 0      | Go to the mycelium, the network and the code across them            |
| g, G   | Go to the first or the last                                         |
| Return | Open what you're on: the map, the network, the code or the launcher |
| y      | Copy the first command of the machine or part you're in             |
| ?, F1  | The list of keys                                                    |

The page is each machine's home, so every other site links a machine
here. Its address for one is `#machine=` and its name, as in
`system/#machine=osprey`: opened there, the cursor starts on that
machine. One past the first 8 starts on the row for the rest, and a
name the page doesn't have is said. While a machine's record may still
be coming, the page waits for it up to five seconds, or until you move.

## What the Data Holds

The system daemon holds three kinds of record. The `summary` has
`machine`, `read_at` (RFC 3339, UTC), `health` (good, watch, bad or
not read), `side` (every part's, in order) and `parts`, the part names
in order. Each `part:<name>` has `part` (machines, mycelium, network or
code), `health`, `says`, `side` (what is worth a look, an item each),
`holds` (what it holds, or null when it wasn't read), `things` (the
keys of its things) and `deeper`: `command`, `site` (from the sites'
root) and `launcher` (its path there). Each `machine:<name>` has
`name`, `health`, `says`, `side`, where its home publishes one,
`address`, and `ties`: its cells, network, code, figures and acts, as
the system skill says them. A tie that is null wasn't read.
`system --json` says the same as one record.

The system asks each owner rather than reading their sources:
`mycelium machines`, `mycelium health`, `net --json` and `repos --json`,
at once, each given ten seconds.
