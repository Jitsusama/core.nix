# Proves, as a generation builds, that a realm's node takes the growing
# percent its conversations are given: a node that refuses it as a bad option
# would end each conversation as it starts, which only a host's journal says
# for a conversation woken in the background. So such a node fails the switch
# by name instead. host.nix names this build in each conversation's unit, and
# conversation-checks.nix builds it with a node that refuses the word and
# with one it can't run.
{ lib, pkgs }:
realm: node: percent:
let
  word = "--heap-growing-percent=${toString percent}";
  given = "jitsusama.pi.hosts.${realm}.conversationHeapGrowingPercent is ${toString percent}";
  # A node the build can't run, as one named outside the store is in its
  # sandbox, is not one that refuses the word, and needs another fix.
  unrun = lib.concatStrings [
    "${given}, but the build proving the realm's node takes it can't run ${node}: "
    "give the realm a node from the store, or unset the percent."
  ];
  refusal = lib.concatStrings [
    "${given}, but ${node} refuses ${word}, so each of the realm's conversations would end as it "
    "starts: unset the percent, or give the realm a node that knows it. What it said is above."
  ];
in
pkgs.runCommandLocal "pi-conversation-${realm}-takes-heap-growth" { } ''
  if ! [ -x ${lib.escapeShellArg node} ]; then
    echo ${lib.escapeShellArg unrun} >&2
    exit 1
  fi
  if ! ${lib.escapeShellArg node} ${lib.escapeShellArg word} -e 0; then
    echo ${lib.escapeShellArg refusal} >&2
    exit 1
  fi
  touch $out
''
