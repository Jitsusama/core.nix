# The checks of daemon-client, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves. The
# daemons' tests that name it run it, so its one check is its lint.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # clippy finds nothing in it.
  daemon-client-passes-clippy = passesClippy "daemon-client-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
