# The checks of shell-preview, which checks.nix gathers with every other
# folder's. It shows the shell nested in a Wayland session, which only Linux
# has.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # shell-preview links each built file the working copy holds back to it,
  # so an edit shows without a build, and copies what the build made, and
  # ends the nested desktop it started however it's ended: its tests run
  # as it builds, the whole program among them, against stand-ins for git,
  # nix and the desktop (tools/shell-preview).
  shell-preview-follows-the-working-copy = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in shell-preview, its tests included.
  shell-preview-passes-clippy = passesClippy "shell-preview-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
