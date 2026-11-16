# Your System, Now

How the whole system stands at once: every machine on the mycelium, the
mycelium itself, the home network and the code, each judged from its own
owner's answer, with the way deeper into each. The page watches the
system daemon's records live over the sites' server, and redraws a
part the moment its owner's answer moves. Nothing on it changes
anything.

## How to Read It

- **One sentence for the whole,** worst first: what is bad, then what's
  worth a look, then what couldn't be read. With nothing amiss, it says
  every part is good.
- **One row per part:** Machines, Mycelium, Network and Code, each with
  a mark, its name, how it stands (Good, Worth a look, Bad or Not read)
  and what it says, in its own owner's words where it can.
- **How each part is judged:**
  - Machines: one out of reach or with a nearly or critically full disk
    is bad; one short of room, with a filling disk, or on battery at 20%
    or less is worth a look.
  - Mycelium: a link that isn't up is bad; presences left behind by
    sessions that ended are worth a look.
  - Network: a source that isn't answering is bad; anything else the
    network names as a concern, other than its ceiling, is worth a look.
  - Code: a tree git still lists but whose folder is gone is worth a
    look.
  - A part whose owner can't be asked is Not read, with why. The whole
    takes the worst part that was read.
- **Beside the list is the part the cursor is on:** what it says, its
  machines where it has any, each linked to its own place, and the ways
  deeper: its site, the command that says more in a terminal, and where
  it is in the launcher.
- **Under the list, when it was read and on which machine,** and
  whether what it shows is live: when the daemon or the sites' server
  can't be reached, it says since when and why, and keeps showing how
  things stood then.

## Keys

| Key       | Does                                                       |
| --------- | ---------------------------------------------------------- |
| j, k      | Step to the next or the last part                          |
| g, G      | Go to the first or the last                                |
| Return    | Open the part's site, or the machine's own place           |
| l         | Go into the part, to step through its machines             |
| -, h      | Back up to the parts                                       |
| y         | Copy the command that says more of the part                |
| ?, F1     | The list of keys                                           |

## What the Data Holds

The system daemon holds three kinds of record. The `summary` has
`machine`, `read_at` (RFC 3339, UTC), `health` (good, watch, bad or
not read) and `parts`, the part names in order. Each `part:<name>` has
`part` (machines, mycelium, network or code), `health`, `says`,
`things` (the keys of its things) and `deeper`: `command`, `site`
(from the sites' root) and `launcher` (its path there). Each
`machine:<name>` has `name`, `health`, `says` and, where its home
publishes one, `address`. `system --json` says the same as one
record.

The system asks each owner rather than reading their sources:
`mycelium machines`, `mycelium health`, `net --json` and `repos --json`,
at once, each given ten seconds.
