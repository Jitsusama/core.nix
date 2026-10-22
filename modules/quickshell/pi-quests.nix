# What the launcher's quests read (quests.rs): every quest in pi's quest
# store, as JSON. The standard library alone, so it builds with rustc and no
# crates, and its tests run as it builds.
{ runCommandCC, rustc }:
runCommandCC "pi-quests" { nativeBuildInputs = [ rustc ]; } ''
  rustc --edition 2024 -D warnings --test -o tests ${./quests.rs}
  ./tests -q
  mkdir -p $out/bin
  rustc --edition 2024 -C opt-level=3 -D warnings -o $out/bin/pi-quests ${./quests.rs}
''
