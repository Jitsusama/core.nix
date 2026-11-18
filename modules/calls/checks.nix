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
  # unless it's asked about here alone, even beside a node that won't
  # answer, a stale or quiet one none, and one declaring no call probe, one
  # whose probe said nothing and a node that won't answer each named and
  # no wait, a socket linked from elsewhere never asked.
  calls-tells-a-call = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in it, its tests included.
  calls-passes-clippy = passesClippy "calls-passes-clippy" (pkgs.callPackage ./package.nix { });
}
