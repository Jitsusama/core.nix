# Proves, as a generation builds, that a realm's node takes the growing
# percent its conversations are given: a node that refuses it as a bad option
# would end each conversation as it starts, which only a host's journal says
# for a conversation woken in the background. So such a node fails the switch
# by name instead. host.nix names this build in each conversation's unit, and
# warns where it stands unproven unless the realm vouches for its node;
# conversation-checks.nix builds it with a node that refuses the word, one
# that fails another way, one that never ends, one that ignores being told
# to end, one killed early, one it can't run, and for a machine it can't
# run. The node is given `seconds` to end, so one that hangs fails the
# switch rather than holding it.
{
  lib,
  pkgs,
  seconds ? 60,
}:
realm: node: percent:
let
  word = "--heap-growing-percent=${toString percent}";
  given = "jitsusama.pi.hosts.${realm}.conversationHeapGrowingPercent is ${toString percent}";
  # A node the build can't run hasn't refused the word, and needs another
  # fix: one named outside the store; one named by a store path without its
  # context, which a sandboxed build can't reach; or a store path, with its
  # context, that names nothing runnable, such as a package's folder or a
  # program it doesn't hold.
  unrun = lib.concatStrings [
    "${given}, but the build proving the realm's node takes it can't run ${node}: give the realm "
    "a node its build can run, a program from the store named with its context, as lib.getExe "
    "gives one, or unset the percent."
  ];
  refusal = lib.concatStrings [
    "${given}, but ${node} refuses ${word}, so each of the realm's conversations would end as it "
    "starts: unset the percent, or give the realm a node that knows it. What it said is above."
  ];
  failed = lib.concatStrings [
    "${given}, but ${node} failed with ${word} other than by refusing it, so the build can't prove "
    "it takes the word: give the realm a node its build can run, or unset the percent. What it "
    "said is above."
  ];
  slow = lib.concatStrings [
    "${given}, but ${node} didn't end within its ${toString seconds}-second limit once run "
    "with ${word}, so the build can't prove it takes the word: give the realm a node that "
    "starts and ends, or unset the percent. What it said is above."
  ];
  # Exit 137 with no signal from timeout is a kill the build didn't send:
  # the kernel's for memory, or another process's. Another try may pass, unlike a refusal.
  killed = lib.concatStrings [
    "${given}, but ${node} was killed before its ${toString seconds}-second limit once run "
    "with ${word}, as the kernel kills a process for memory or another process can, so the "
    "build can't prove it takes the word: build again once memory is free. What it said is "
    "above."
  ];
  unproven = lib.concatStrings [
    "${given}, but this machine is built on another platform, whose builder can't run ${node}, so "
    "its switch doesn't prove the node takes ${word}, and a node that refuses it would end each "
    "of the realm's conversations as it starts: build the machine on its own platform, unset "
    "the percent, or name its node in the realm's conversationHeapGrowthVouchedNode once it is "
    "known to take it."
  ];
  name = "pi-conversation-${realm}-takes-heap-growth";
in
# A machine built elsewhere, whose programs its builder can't run, can't
# have its node run here either: the proof stands unproven and says so for
# the machine's warnings, rather than failing the switch as if the node
# refused the word.
if pkgs.stdenv.buildPlatform.canExecute pkgs.stdenv.hostPlatform then
  pkgs.runCommandLocal name { } ''
    ${lib.getExe pkgs.bash} ${./taken.sh} ${
      lib.escapeShellArgs [
        node
        word
        (toString seconds)
        unrun
        refusal
        failed
        slow
        killed
      ]
    } "$out"
  ''
else
  pkgs.runCommandLocal name { passthru = { inherit unproven; }; } ''
    echo ${lib.escapeShellArg "${node} was not run with ${word}: this machine is built elsewhere."} > "$out"
  ''
