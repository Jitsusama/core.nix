# The checks of bluetooth-links, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves. It
# speaks Linux's Bluetooth sockets, so both are Linux's alone.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # Its tests run as it builds: the kernel's lists of adapters and
  # connections are read, keeping only the data links that are up; each
  # command is written as the controller takes it, and its answer picked
  # out of the events, refused or not; the signal, link quality, power and
  # channel map are read from reply bytes, a failed one leaving its field
  # null; nothing connected is written once; and the file is replaced
  # whole, readable by everyone.
  bluetooth-links-reads-the-replies = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in it, its tests included.
  bluetooth-links-passes-clippy = passesClippy "bluetooth-links-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
