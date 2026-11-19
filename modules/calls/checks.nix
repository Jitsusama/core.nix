# The checks of calls, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # calls' tests run as it builds: a call app's running recording is seen
  # and goes when it idles or its node does, while PipeWire's own filters,
  # dictation's recorder, the camera's videos, a sink's monitor and
  # playback are left out; and a-call-is-on says a call once the same node
  # still records two seconds on, in any account's PipeWire, none for a
  # moment's recording or with nobody logged in, and that it can't tell
  # when a PipeWire won't answer; and that a fresh call on another of the
  # person's machines, as a stand-in mycelium node tells it, is a call
  # unless it's asked about here alone, which asks no node, a stale one
  # none; one declaring no call probe, one whose probe said nothing, one
  # dated ahead, and a node that won't answer, floods or has a full queue
  # are each named and no wait; every node is asked at once, so mute ones
  # starve none and a mute one beside PipeWire's second look still answers
  # within nine seconds, while one slowed to three and a half is heard; an
  # account's first 16 nodes are asked, a socket linked from elsewhere
  # never, and a mistyped flag is refused.
  calls-tells-a-call = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in it, its tests included.
  calls-passes-clippy = passesClippy "calls-passes-clippy" (pkgs.callPackage ./package.nix { });
}
