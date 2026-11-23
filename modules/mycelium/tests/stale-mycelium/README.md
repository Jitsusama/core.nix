# Stand-ins for a Stale Mycelium

Mycelium sources as a machine overriding core's input might have them, each
holding only the two files `carry-guards.nix` reads, so `checks.nix` can show
`easesCarry` is dropped, with a warning, against a mycelium without the
guards #381 brought:

- `without/`: from before #381, with both files but neither guard's line.
- `commented/`: naming both guards only in comments, which don't count.
- `half/`: holding one guard's line but not the other.
- `reshaped/`: after #381, with both lines written in words the patterns
  don't match.
