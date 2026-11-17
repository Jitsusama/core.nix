# The checks of daemon-client, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves. The
# daemons' tests that name it run it too.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # Its test runs as it builds: a client asking a socket that's on the
  # disk but not listened on yet waits for it, as a daemon starting under
  # load gives.
  daemon-client-waits-for-a-socket-to-listen = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in it.
  daemon-client-passes-clippy = passesClippy "daemon-client-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
