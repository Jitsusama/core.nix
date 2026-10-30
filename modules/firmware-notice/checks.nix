# The checks of firmware-notice, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# fwupd is Linux's, and so is the notice.
onLinux {
  # firmware-notice's tests run as it builds: each device offered an update
  # says its newest release and whether it finishes at a restart, nothing
  # offered says nothing, fwupd not answering keeps what was said, and a
  # release is said once, kept only while it's offered.
  firmware-notice-says-each-release-once = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in firmware-notice, its tests included.
  firmware-notice-passes-clippy = passesClippy "firmware-notice-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
