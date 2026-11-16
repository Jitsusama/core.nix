# Code

Every repository on the machine as a grove: which of its trees are being
written, which of its changes wait and on what, the cells at work in each,
and what's worth a look. The page watches the repos daemon's records live
over the sites' server, and redraws the moment git writes a branch, a tree
is cut or a change's checks move. Nothing on it changes anything.

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
  its name, its size in trees, the cells at work and when it was last
  written.
- **Beside the list, the repository the cursor is on:** its trunk, its
  checkouts, its changes with how their checks and reviews stand, and its
  trees by season: Written Today, This Week, Resting and Gone, the last two
  folded until asked for. Within a season they go by branch, so a tree keeps
  its place while it's written in.
- **Under it all,** whether what it shows is live, and since when it isn't.

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

The repos daemon (`repos serve`) holds five kinds of record. The sole
`reading` has `machine`, `roots` and `sources`, each source with its
`name`, `what` and `unread` list. Each `repository:<key>` has `name`,
`owner`, `remotes`, `trunk`, `written_at` and `read_through`. Each
`checkout:<path>` has `repository`, `path`, `bare`, `head` and
`written_at`. Each `tree:<path>` has `repository`, `checkout`, `path`,
`snapshot`, `present`, `head`, `written_at` and `change`. Each
`change:<key>#<number>`, or `@<branch>` before GitHub numbers it, has its
`title`, `branch`, `checks` and `review`. Times are RFC 3339, UTC.

`repos --json` says the same as one record, and `repos <name>` one
repository.

The cells at work come from the machine's mycelium node, its `cell` kind:
each cell's `name`, `cwd`, `title`, `standing` and `networks`. A cell is
at work in the deepest tree or checkout its `cwd` lies in.
