# Stand-ins for a Stale Mycelium

Mycelium sources as a machine overriding core's input might have them, each
holding the two files `carry-guards.nix` reads, where it has them, and a
note. `checks.nix` shows that `easesCarry` is dropped against each of the
six that lack a guard mycelium's #381 brought, with a warning naming which
files lack one and how. Four of them lack it in one file alone, so a
pattern made too loose lets its own case through:

- `without/`: from before #381, with both files but neither guard's line.
- `older/`: from before easing, with neither file.
- `half/`: with `driven` in `ease.rs` but no `leads` in `easing.rs`.
- `other-half/`: with `leads` but no `driven`.
- `commented/`: naming `driven` only in line comments, with a real `leads`.
- `commented-leads/`: naming `leads` only in line comments, with a real
  `driven`.

And one it keeps, so a pattern made too strict refuses its case:

- `reshaped/`: after #381, with both lines in forms the patterns still
  accept, a `pub(crate)` field and a `fn leads` with a lifetime, so a node
  built from it carries.

Two more hold only the `main.rs` that `eases.nix` reads for the flag
mycelium's #441 brought, `--eases-onto`, and `older/` stands for a source
without it. A node is named the workers its machine names only from the
first:

- `names-workers/`: after #441, declaring the flag's field.
- `commented-onto/`: naming that field only in a line comment.
