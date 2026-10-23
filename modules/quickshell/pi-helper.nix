# What the shell reads through a helper of its own, each a single Rust
# file on the standard library alone, so it builds with rustc and no
# crates, and its tests run as it builds: pi-quests (quests.rs), every
# quest in pi's store, pi-runs (runs.rs), pi's fleets and review rounds,
# and tuple-people (../tuple/people.rs), Tuple's contacts.
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
