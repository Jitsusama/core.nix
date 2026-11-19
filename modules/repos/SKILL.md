---
name: repos
description: >
  Every repository on this machine, each once, with its checkouts and the
  trees cut from them, read from git's own files with `repos`. Use when
  asked which repositories Joel has, where one is checked out, what its
  trunk or remote is, which trees or worktrees exist for it and what each
  is at, whether a tree for a branch is already there, which trees are gone
  from the disk, what was worked on lately, or which changes are open on a
  repository and how their checks and review stand; and before cloning or
  cutting a tree, to find the copy already there.
---

# repos

`repos` lists every repository on the machine once, however many copies of
it lie on the disk: each checkout of it, and every tree cut from each,
wherever that tree's folder is. It reads git's own files each time it runs,
in well under a second, so its answer is the disk as it is now. Prefer it to
walking the disk with `find`, which misses trees whose folder lies outside
where you look, or to `git worktree list` in each checkout, which needs every
checkout found first.

It only reads. It never fetches, prunes or writes to a repository; a tree is
still cut and given back through the `work` tool, never `git worktree`.

## The Words

The words are the glossary's Code context (docs/glossary.md in core.nix), and
an answer about repositories uses them as they are:

- A **repository** is one history, listed once; its **key** names it once,
  as `github:owner/name`, `gitlab:owner/name`, `<host>:<path>`, or
  `local:<folder>` for one with no remote. Never "repo".
- A **checkout** is a repository's whole copy, which trees are cut from; a
  **bare** one holds no files of its own. Never "clone".
- A **tree** is a folder of its files to work in: a **worktree** is at a
  **branch**, a **snapshot** pinned at a **commit**. One its checkout still
  lists whose folder isn't there is **gone** (git's prunable).
- The **trunk** is the branch its changes merge into, as its remote names
  it. Never "default branch".
- A **change** is work put up for review on any source, a **check** what
  proves it. Never "PR", "merge request", "CI" or "pipeline".

## Asking

Names match in any case, by name, `owner/name` or the whole key. Add
`--json` for the record, whose `schema` names its shape.

```bash
repos                        # each repository: trees, checkouts, written
repos core.nix               # one: remotes, trunk, each checkout, newest trees
repos core.nix trees         # every tree of it, newest first
repos core.nix changes       # its open changes, updated last first
repos --json                 # the whole record, repos.repositories/1
repos core.nix --json        # one, repos.repository/1
repos core.nix trees --json  # its trees, repos.trees/1
repos core.nix changes --json  # its open changes, repos.changes/1
```

A name two repositories share is refused with exit 64 and each one's key, so
ask again by `owner/name` or key. An unknown name exits 66 and names the
nearest. `repos --help` lists both.

## Reading the Record

`repos.repositories/1` is `{schema, read_at, machine, roots, sources,
repositories}`. Each repository is `{key, name, owner, remotes [{name, url}],
trunk, written_at, changes, checkouts}`, the one written to last first; each
change `{title, author, draft, branch, base, commit, updated_at, checks
{state, passed, failed, pending}, review, github {number, url}}`, where a
check's `state` is passing, failing, pending or none and `review` is approved,
changes_asked, waiting or none; each checkout
`{path, bare, head {branch, commit}, written_at, weight, branches, config_bytes,
trees}`; each tree
`{path, kind ("worktree" or "snapshot"), present, head, written_at}`. A bare
checkout's `head` is null, and a `branch` is null where the head is a bare
commit. A checkout's `weight` is its git store, `{loose, estimated,
loose_bytes, packs, packed_bytes, due}`. Its bytes are the room the store takes
on the disk. Loose objects are counted in every folder when few, and where
`estimated` reckoned from one of the 256, as git's auto-gc does. `due` says
git's automatic gc, at the checkout's own config, would gather it now. The
weight is left out where the store couldn't be read. `branches` counts its local
branches, loose, packed or in its reftable, each once, and `config_bytes` is its
config file's length, without files it includes, which every git command there
reads; either is left out where it couldn't be read, and is null in the
daemon's checkout records, never 0. Paths are whole, times
RFC 3339, and an unread one is null.
`repos NAME --json` holds its one under `repository`, and
`repos NAME trees --json` its trees, newest first, each naming its
`checkout`, under `trees`.

```bash
# Where a repository is checked out.
repos core.nix --json | jq -r '.repository.checkouts[].path'
# Whether a tree for a branch is already there, and where.
repos core.nix trees --json \
  | jq -r '.trees[] | select(.head.branch == "fix/x" and .present) | .path'
# Open changes whose checks fail, and where each is.
repos core.nix changes --json \
  | jq -r '.changes[] | select(.checks.state == "failing") | .github.url'
# Trees gone from the disk, which the checkout still lists.
repos --json | jq -r '.repositories[] | .key as $k
  | .checkouts[].trees[] | select(.present | not) | "\($k) \(.path)"'
```

## What It Doesn't Say

- Only checkouts under its roots are found, up to four folders deep
  (`REPOS_ROOTS`, from `jitsusama.repos.roots`); a tree is found from its
  checkout wherever it lies. A repository checked out nowhere under them
  isn't listed.
- Changes are GitHub's alone, asked for by a timer every five minutes, so
  they may be that old: the github source's `read_at` says when, and
  `repos refresh` asks now. A change's diff and conversation are read with
  the harness's review tools.
- One machine's disk only: `machine` says which.
