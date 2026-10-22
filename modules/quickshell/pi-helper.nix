# What the launcher reads of pi: one of its helpers, each a single Rust
# file on the standard library alone, so it builds with rustc and no
# crates, and its tests run as it builds: pi-quests (quests.rs), every
# quest in the store, and pi-runs (runs.rs), the fleets and review rounds.
{
  runCommandCC,
  rustc,
  name,
  source,
}:
runCommandCC name { nativeBuildInputs = [ rustc ]; } ''
  rustc --edition 2024 -D warnings --test -o tests ${source}
  ./tests -q
  mkdir -p $out/bin
  rustc --edition 2024 -C opt-level=3 -D warnings -o $out/bin/${name} ${source}
''
