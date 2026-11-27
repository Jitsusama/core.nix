# What Your Sessions Murmur

What this machine's cells murmured into the quests' webs over the last
day: each murmur's words, the cell that said it, the quests it is tied
to, and how it was settled once it is. The page watches the mycelium
node's murmur records live over the sites' server, so a murmur arrives,
ties or settles the moment it is said. Nothing on it changes anything;
`mycelium web` in a terminal says, ties and settles.

## How to Read It

- **The flock, at dusk:** each murmur still in the air is a bird in the
  flock of the cell that said it, and a hairline runs from it to the
  flock of each quest it is tied to whose cell murmurs here too. A settled murmur has come down to
  roost on the wire below, a body at rest rather than wings, so which is
  which is told by shape. The first 64 murmurs are drawn; the list holds
  every one.
- **Under the title, which never moves, one sentence for the whole:** how
  many were murmured, and how many are still in the air, each count
  opening the murmurs it counts; then the newest still in the air, quoted
  with the quest its cell works on.
- **One row per murmur,** newest first: its words, the cell that said
  it and when, the quests it is tied to, and how it was settled.
- **Beside the sentence, whether what it shows is live:** when the node
  or the sites' server can't be reached, it says since when and why, and
  keeps showing how things stood then.

## Keys

| Key   | Does                                    |
| ----- | --------------------------------------- |
| j, k  | Step to the next or the last murmur     |
| g, G  | Go to the newest or the oldest          |
| ?, F1 | The list of keys                        |

## What the Data Holds

The node serves one `murmur` record for each murmur this machine's
cells said over the last day, keyed `murmur:<cell>:<at>`; an older one
leaves the page as it leaves the node. Each has `writer` (the cell that
said it), `at` (when, in seconds), `text` (its words), `tied_to` (the
quests it is tied to, by their ids), `purpose` (what its cell works on,
title and summary) and `settled` (how it was settled, or null while it
is still in the air). The page asks for those fields alone.
