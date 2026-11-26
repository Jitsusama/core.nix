# Proves, as a generation builds, that a realm's node takes the growing
# percent its conversations are given: a node that refuses it as a bad option
# would end each conversation as it starts, which only a host's journal says
# for a conversation woken in the background. So such a node fails the switch
# by name instead. host.nix names this build in each conversation's unit, and
# conversation-checks.nix builds it with a node that refuses the word, one
# that fails another way, one it can't run, and for a machine it can't run.
{ lib, pkgs }:
realm: node: percent:
let
  word = "--heap-growing-percent=${toString percent}";
  given = "jitsusama.pi.hosts.${realm}.conversationHeapGrowingPercent is ${toString percent}";
  # A node the build can't run, as one named outside the store, or by a
  # store path with no context, is in its sandbox, is not one that refuses
  # the word, and needs another fix.
  unrun = lib.concatStrings [
    "${given}, but the build proving the realm's node takes it can't run ${node}: give the realm "
    "a node its build can reach, from the store and with its context, as lib.getExe gives one, "
    "or unset the percent."
  ];
  refusal = lib.concatStrings [
    "${given}, but ${node} refuses ${word}, so each of the realm's conversations would end as it "
    "starts: unset the percent, or give the realm a node that knows it. What it said is above."
  ];
  failed = lib.concatStrings [
    "${given}, but ${node} failed with ${word} other than by refusing it, as a node that can't "
    "start would: what it said is above."
  ];
  name = "pi-conversation-${realm}-takes-heap-growth";
in
# A machine built elsewhere, whose programs its builder can't run, can't
# have its node run here either: the proof stands as unproven, rather than
# failing the switch as if the node refused the word.
if pkgs.stdenv.buildPlatform.canExecute pkgs.stdenv.hostPlatform then
  pkgs.runCommandLocal name { } ''
    ${lib.getExe pkgs.bash} ${./taken.sh} ${
      lib.escapeShellArgs [
        node
        word
        unrun
        refusal
        failed
      ]
    } "$out"
  ''
else
  pkgs.writeText name "${node} was not run with ${word}: this machine is built elsewhere.\n"
