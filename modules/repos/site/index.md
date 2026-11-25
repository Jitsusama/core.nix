# Code

Every repository on the machine as a grove: which of its trees are being
written, which of its changes wait and on what, the cells at work in each,
and what's worth a look. The page watches the repos daemon's records live
over the sites' server, and redraws the moment git writes a branch, a tree
is cut or a change's checks move. Nothing on it changes anything: what's
done to a repository is done from its row in the launcher's Code, which
its Open it in the launcher opens, the row selected. There it's pulled,
its gone trees forgotten, keeping locked ones and every branch, and
none while a gone tree holds commits no branch has; and its store
gathered when git would gather it, each act offered only where it has
something to do.

## How to Read It

- **One sentence for the whole:** how many repositories there are, how many
  are growing (a tree written today), then at most two things worth a look,
  worst first, and which repository grew most today.
- **What's worth a look,** in this order: cells waiting on Joel, changes
  whose checks fail, changes with changes asked for, trees git still lists
  whose folders are gone, and ten or more trees resting over a week.
- **The grove:** a tree for each repository, kept in its place by name. Its
  crown grows with the trees cut from it. Its leaves are those trees:
  filled when written today, outlined this week; resting ones aren't drawn.
  Buds on the rim are its changes: closed and dashed while checks run, open
  with a centre once they pass, drawn heavier in rust when one fails.
  Stumps at the ground are trees whose folders are gone, and
  threads under it are the cells at work there.
- **The list:** each repository as a row, its marks in a gutter (its
  season and its worst change, ringed while it waits on a review), then
  its name, its size in trees, the cells at work, when it was last
  written, what its git stores weigh, with "upkeep due" when git would
  gather one now, and what works in it: its share of a core over the last
  minute and the memory its processes hold.
- **Beside the list, the repository the cursor is on:** its trunk, the
  way to it in the launcher, its checkouts with what each store is made
  of and what works in each, its
  changes with how their
  checks and reviews stand, and its trees by season: Written Today, This
  Week, Resting and Gone, the last two folded until asked for. The
  others list at most twelve, those cells work in or a change names
  first, then the newest, and count the rest, snapshots no cell works
  in always among them, until z or a season's button lists every tree;
  a link to one tree opens only its season, and the trees the cursor
  has stood on stay listed while the repository is open.
  Within a season they go by branch, so a tree keeps its place while
  it's written in. Under each tree or checkout are the cells at work in it, each
  linked home and followed by the other repositories it works across,
  each linked to where it's shown here, so its work can be followed on.
- **How they join,** under the work: a row for each repository a cell's
  work ties to another, in the grove's order, and beside the rows an arc
  for each tie with a thread for each cell (at most six drawn, every one
  counted in words). A tie stands when a cell at work in one repository
  names the other's network. The threads of the row the keys rest on are
  drawn brighter, and the row says what it's joined to and by how many.
  j and k, or up and down, step through the rows, and Home and End go to
  the first and last; Return opens the one the keys rest on. Shown only
  while some cell's work spans two repositories.
- **Under it all,** whether what it shows is live, and since when it isn't.

What works in a repository is read once a minute from Linux's `/proc`:
each of Joel's processes whose working folder lies in a checkout or tree
counts for the deepest one holding it, its memory its proportional share,
so libraries several share count once between them, and its processor
time with its children's once it has waited on them, so a build's
compilers count in the cargo that ran them, and a child already counted
isn't counted again. Time counts only through a process working in a
folder at both readings: one working out of every tree, or under `sudo`,
counts for none of that time, and one waited on by a process outside
every folder, as the user's service manager, loses its last minute. The
share is of the waking time between readings,
so a suspend leaves it as it was. A build the Nix daemon runs
for one works as its own user in its own folder, so it's counted
nowhere, and a Mac says none of it. Where the daemon can't say, the page
says nothing of it, and the rest of the page stands as it was; until its
reading of `/proc` first answers, and once its last reading is three
minutes old with none newer, each repository says its processes are
unread rather than that nothing works in it. So does a repository with a
tree newer than the last reading, until the next looks in it.

## Keys

| Key     | Does                                                              |
| ------- | ----------------------------------------------------------------- |
| j, k    | Step to the next or the last repository, or tree once stepped in  |
| h, l    | Walk the grove to the repository beside this one                  |
| g, G    | Go to the first or the last                                       |
| Return  | Step into the repository, to walk its trees                       |
| -       | Step back out to the repositories                                 |
| /       | Find a repository by its name                                     |
| Esc     | Let go of what was found, to see every repository again           |
| o       | Open the tree's change, or the repository where GitHub keeps it   |
| z       | Open or fold the trees resting and gone                           |
| y       | Copy the tree's folder, or the repository's checkout              |
| ?, F1   | The list of keys                                                  |

## What the Data Holds

The repos daemon (`repos serve`) holds six kinds of record. The sole
`reading` has `machine`, `roots` and `sources`, each source with its
`name`, `what` and `unread` list. Each `repository:<key>` has `name`,
`owner`, `remotes`, `trunk`, `written_at` and `read_through`. Each
`checkout:<path>` has `repository`, `path`, `bare`, `head`,
`written_at`, `weight`, `branches` and `config_bytes`, the last two its
local branch count and its config's bytes, null where they couldn't be
read. Each `tree:<path>` has `repository`, `checkout`, `path`,
`snapshot`, `present`, `head`, `written_at` and `change`. Each
`change:<key>#<number>`, or `@<branch>` before GitHub numbers it, has its
`title`, `branch`, `checks` and `review`. Each `using:<key>` has its
`repository`, its `processes`, `memory` in bytes and `cpu` as a share of
one core, null until two readings are a minute apart and after a reading
held up, all three null while the reading has stopped answering, and
`folders`, each checkout or tree anything works in with its `path` and
the same three.
Times are RFC 3339, UTC.

`repos --json` says the same as one record, and `repos <name>` one
repository.

The cells at work come from the machine's mycelium node, its `cell` kind:
each cell's `name`, `cwd`, `title`, `standing` and `networks`. A cell is
at work in the deepest tree or checkout its `cwd` lies in.
