# Proves, as a generation builds, that a realm's node takes the growing
# percent its conversations are given: a node that refuses it as a bad option
# would end each conversation as it starts, which only a host's journal says
# for a conversation woken in the background. So such a node fails the switch
# by name instead. host.nix names this build in each conversation's unit, and
# conversation-checks.nix builds it with a node that refuses the word.
{ lib, pkgs }:
realm: node: percent:
let
  word = "--heap-growing-percent=${toString percent}";
  refusal = lib.concatStrings [
    "jitsusama.pi.hosts.${realm}.conversationHeapGrowingPercent is ${toString percent}, "
    "but ${node} refuses ${word}, so each of the realm's conversations would end as it "
    "starts: unset the percent, or give the realm a node that knows it."
  ];
in
pkgs.runCommandLocal "pi-conversation-${realm}-takes-heap-growth" { } ''
  if ! ${lib.escapeShellArg node} ${lib.escapeShellArg word} -e 0; then
    echo ${lib.escapeShellArg refusal} >&2
    exit 1
  fi
  touch $out
''
