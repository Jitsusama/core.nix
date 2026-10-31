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
  # when a PipeWire won't answer.
  calls-tells-a-call = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in it, its tests included.
  calls-passes-clippy = passesClippy "calls-passes-clippy" (pkgs.callPackage ./package.nix { });
}
