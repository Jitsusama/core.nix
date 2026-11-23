# Stand-ins for a Stale Mycelium

Mycelium sources as a machine overriding core's input might have them, each
holding only the two files `carry-guards.nix` reads. `checks.nix` shows that
`easesCarry` is dropped, with a warning naming what was missing, against
each of the five that lack a guard mycelium's #381 brought. Each of those
fails on one file alone, so a pattern made too loose lets its own case
through:

- `without/`: from before #381, with both files but neither guard's line.
- `half/`: with `driven` in `ease.rs` but no `leads` in `easing.rs`.
- `other-half/`: with `leads` but no `driven`.
- `commented/`: naming `driven` only in line comments, with a real `leads`.
- `commented-leads/`: naming `leads` only in line comments, with a real
  `driven`.

And one it keeps, so a pattern made too strict refuses its case:

- `reshaped/`: after #381, with both lines in forms the patterns still
  accept, a `pub(crate)` field and a `fn leads` with a lifetime, so a node
  built from it carries.
