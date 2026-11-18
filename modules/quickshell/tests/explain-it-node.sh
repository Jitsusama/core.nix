# shellcheck shell=bash disable=SC2154 # $MYCELIUM_ASKED comes from the check, in quickshell/checks/home.nix.
# A stand-in for mycelium as Explain It asks it, `mycelium --realm <realm>
# cells <ADDRESS> -o json`, for tests/explainer.qml: it writes down each
# asking in $MYCELIUM_ASKED, says HARBOR's session, answers SLOW's late,
# never answers HANG, can't say DOWN, finds only another session by
# TITLED's title words, and knows no other, saying so as mycelium does.
printf '%s\n' "$*" >> "$MYCELIUM_ASKED"
cell() {
  printf '{"kind":"cell","type":"session","name":"%s","title":"%s",' "$1" "$2"
  printf '"quest":"QEST-20261116-HARBOR","cwd":"/home/j/src/rows","repo":"rows"}\n'
}
case "$4" in
  HARBOR) cell HARBOR "Tidy the Rows" ;;
  # A node slow to answer: the test asks another meanwhile.
  SLOW) sleep 1 && cell SLOW "Slow Work" ;;
  # A node that never answers, past the explainer's patience.
  HANG) exec sleep 30 ;;
  DOWN) echo "mycelium: the node isn't answering" >&2 && exit 1 ;;
  TITLED) cell OTHER1 "Titled Work" ;;
  *) echo "mycelium: no cell is named $4" >&2 && exit 1 ;;
esac
