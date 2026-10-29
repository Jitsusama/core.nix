# The checks of slack, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# slack-people serves the Linux desktop's shell, and builds only on Linux.
onLinux {
  # slack-people's tests run as it builds: who Slack knows by an address is
  # read from its answer, one it knows nobody by said so, a sign-in it
  # refuses said once and nobody more asked about, each address asked about
  # once with the harness's sign-in, nothing but a look-up ever asked, and
  # with no sign-in nothing asked at all. A stand-in on the machine answers
  # for Slack.
  slack-finds-who-it-knows = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in slack-people, its tests included.
  slack-passes-clippy = passesClippy "slack-passes-clippy" (pkgs.callPackage ./package.nix { });
}
